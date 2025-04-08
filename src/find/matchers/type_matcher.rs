// Copyright 2017 Google Inc.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::error::Error;

use super::{FileType, Follow, Matcher, MatcherIO, WalkEntry};

type TypeList = Vec<FileType>;

/// This matcher checks the type of the file.
pub struct TypeMatcher {
    file_type: TypeList,
}

fn parse(type_string: &str, mode: &str) -> Result<FileType, Box<dyn Error>> {
    let file_type = match type_string {
        "f" => FileType::Regular,
        "d" => FileType::Directory,
        "l" => FileType::Symlink,
        "b" => FileType::BlockDevice,
        "c" => FileType::CharDevice,
        "p" => FileType::Fifo, // named pipe (FIFO)
        "s" => FileType::Socket,
        // D: door (Solaris)
        "D" => {
            #[cfg(not(target_os = "solaris"))]
            {
                return Err(From::from(format!("{mode} D is not supported because Solaris doors are not supported on the platform find was compiled on.")));
            }
            #[cfg(target_os = "solaris")]
            {
                return Err(From::from(format!(
                    "Type argument {type_string} not supported yet"
                )));
            }
        }
        "" => {
            return Err(From::from(format!(
                "Arguments to {mode} should contain at least one letter"
            )))
        }
        _ => {
            return Err(From::from(format!(
                "Unrecognised type argument {type_string}"
            )))
        }
    };
    Ok(file_type)
}

impl TypeMatcher {
    pub fn new(type_string: &str) -> Result<Self, Box<dyn Error>> {
        let main_file_type = type_creator(type_string, "-type")?;
        Ok(Self {
            file_type: main_file_type,
        })
    }
}

impl Matcher for TypeMatcher {
    fn matches(&self, file_info: &WalkEntry, _: &mut MatcherIO) -> bool {
        self.file_type
            .iter()
            .any(|entry| *entry == file_info.file_type())
    }
}

/// Like [TypeMatcher], but toggles whether symlinks are followed.
pub struct XtypeMatcher {
    file_type: TypeList,
}

impl XtypeMatcher {
    pub fn new(type_string: &str) -> Result<Self, Box<dyn Error>> {
        let main_file_type = type_creator(type_string, "-xtype")?;
        Ok(Self {
            file_type: main_file_type,
        })
    }
}
impl Matcher for XtypeMatcher {
    fn matches(&self, file_info: &WalkEntry, _: &mut MatcherIO) -> bool {
        let follow = if file_info.follow() {
            Follow::Never
        } else {
            Follow::Always
        };

        let file_type_result = follow
            .metadata(file_info)
            .map(|m| m.file_type())
            .map(FileType::from);

        for expected_type in &self.file_type {
            if let Ok(file_type) = file_type_result {
                if file_type == *expected_type {
                    return true;
                }
            } else if let Err(e) = &file_type_result {
                // Since GNU find 4.10, ELOOP will match -xtype l
                if e.is_loop() && *expected_type == FileType::Symlink {
                    return true;
                }
            }
        }
        false
    }
}

fn type_creator(type_string: &str, mode: &str) -> Result<TypeList, Box<dyn Error>> {
    if type_string.contains(',') {
        let mut seen = std::collections::HashSet::new();

        let file_type = type_string
                .split(',')
                .map(|s| {
                    let trimmed = s.trim();
                    if trimmed.is_empty() {
                        Err(From::from(format!("find: Last file type in list argument to {mode} is missing, i.e., list is ending on: ','")))
                    } else if !seen.insert(trimmed) {
                        return Err(From::from(format!(
                            "Duplicate file type '{s}' in the argument list to {mode}"
                        )));
                    } else {
                        parse(trimmed,mode)
                    }
                })
                .collect::<Result<Vec<FileType>, _>>()?;
        Ok(file_type)
    } else {
        if type_string.len() > 1 {
            return Err(From::from(format!(
                "Must separate multiple arguments to {mode} using: ','"
            )));
        }
        let file_type = vec![parse(type_string, mode)?];
        Ok(file_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::find::matchers::tests::get_dir_entry_for;
    use crate::find::tests::FakeDependencies;
    use std::io::ErrorKind;

    #[cfg(unix)]
    use std::os::unix::fs::symlink;

    #[cfg(unix)]
    use crate::find::matchers::tests::get_dir_entry_follow;

    #[cfg(windows)]
    use std::os::windows::fs::{symlink_dir, symlink_file};

    #[test]
    fn file_type_matcher() {
        let file = get_dir_entry_for("test_data/simple", "abbbc");
        let dir = get_dir_entry_for("test_data", "simple");
        let deps = FakeDependencies::new();

        let matcher = TypeMatcher::new("f").unwrap();
        assert!(!matcher.matches(&dir, &mut deps.new_matcher_io()));
        assert!(matcher.matches(&file, &mut deps.new_matcher_io()));
    }

    #[test]
    fn dir_type_matcher() {
        let file = get_dir_entry_for("test_data/simple", "abbbc");
        let dir = get_dir_entry_for("test_data", "simple");
        let deps = FakeDependencies::new();

        let matcher = TypeMatcher::new("d").unwrap();
        assert!(matcher.matches(&dir, &mut deps.new_matcher_io()));
        assert!(!matcher.matches(&file, &mut deps.new_matcher_io()));
    }

    // git does not translate links (in test_data) to Windows links
    // so we have to create links in test
    #[test]
    fn link_type_matcher() {
        #[cfg(unix)]
        {
            if let Err(e) = symlink("abbbc", "test_data/links/link-f") {
                assert!(
                    e.kind() == ErrorKind::AlreadyExists,
                    "Failed to create sym link: {e:?}"
                );
            }
            if let Err(e) = symlink("subdir", "test_data/links/link-d") {
                assert!(
                    e.kind() == ErrorKind::AlreadyExists,
                    "Failed to create sym link: {e:?}"
                );
            }
        };
        #[cfg(windows)]
        let _ = {
            if let Err(e) = symlink_file("abbbc", "test_data/links/link-f") {
                assert!(
                    e.kind() == ErrorKind::AlreadyExists,
                    "Failed to create sym link: {:?}",
                    e
                );
            }
            if let Err(e) = symlink_dir("subdir", "test_data/links/link-d") {
                assert!(
                    e.kind() == ErrorKind::AlreadyExists,
                    "Failed to create sym link: {:?}",
                    e
                );
            }
        };

        let link_f = get_dir_entry_for("test_data/links", "link-f");
        let link_d = get_dir_entry_for("test_data/links", "link-d");
        let file = get_dir_entry_for("test_data/links", "abbbc");
        let dir = get_dir_entry_for("test_data", "links");
        let deps = FakeDependencies::new();

        let matcher = TypeMatcher::new("l").unwrap();
        assert!(!matcher.matches(&dir, &mut deps.new_matcher_io()));
        assert!(!matcher.matches(&file, &mut deps.new_matcher_io()));
        assert!(matcher.matches(&link_f, &mut deps.new_matcher_io()));
        assert!(matcher.matches(&link_d, &mut deps.new_matcher_io()));
    }

    #[cfg(unix)]
    #[test]
    fn unix_extra_type_matcher() {
        let file = get_dir_entry_for("test_data/simple", "abbbc");
        let dir = get_dir_entry_for("test_data", "simple");
        let deps = FakeDependencies::new();

        for typ in &["b", "c", "p", "s"] {
            let matcher = TypeMatcher::new(typ).unwrap();
            assert!(!matcher.matches(&dir, &mut deps.new_matcher_io()));
            assert!(!matcher.matches(&file, &mut deps.new_matcher_io()));
        }
    }

    #[test]
    fn cant_create_with_invalid_pattern() {
        let result = TypeMatcher::new("xxx");
        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[test]
    fn xtype_file() {
        let matcher = XtypeMatcher::new("f").unwrap();
        let deps = FakeDependencies::new();

        let entry = get_dir_entry_follow("test_data/links", "abbbc", Follow::Never);
        assert!(matcher.matches(&entry, &mut deps.new_matcher_io()));

        let entry = get_dir_entry_follow("test_data/links", "link-f", Follow::Never);
        assert!(matcher.matches(&entry, &mut deps.new_matcher_io()));

        let entry = get_dir_entry_follow("test_data/links", "link-f", Follow::Always);
        assert!(!matcher.matches(&entry, &mut deps.new_matcher_io()));
    }

    #[cfg(unix)]
    #[test]
    fn xtype_link() {
        let matcher = XtypeMatcher::new("l").unwrap();
        let deps = FakeDependencies::new();

        let entry = get_dir_entry_follow("test_data/links", "abbbc", Follow::Never);
        assert!(!matcher.matches(&entry, &mut deps.new_matcher_io()));

        let entry = get_dir_entry_follow("test_data/links", "link-f", Follow::Never);
        assert!(!matcher.matches(&entry, &mut deps.new_matcher_io()));

        let entry = get_dir_entry_follow("test_data/links", "link-missing", Follow::Never);
        assert!(matcher.matches(&entry, &mut deps.new_matcher_io()));

        let entry = get_dir_entry_follow("test_data/links", "link-notdir", Follow::Never);
        assert!(matcher.matches(&entry, &mut deps.new_matcher_io()));

        let entry = get_dir_entry_follow("test_data/links", "link-f", Follow::Always);
        assert!(matcher.matches(&entry, &mut deps.new_matcher_io()));
    }

    #[cfg(unix)]
    #[test]
    fn xtype_loop() {
        let matcher = XtypeMatcher::new("l").unwrap();
        let entry = get_dir_entry_for("test_data/links", "link-loop");
        let deps = FakeDependencies::new();
        assert!(matcher.matches(&entry, &mut deps.new_matcher_io()));
    }

    #[test]
    fn chained_arguments_type() {
        assert!(TypeMatcher::new("").is_err());
        assert!(TypeMatcher::new("f,f").is_err());
        assert!(TypeMatcher::new("f,").is_err());
        assert!(TypeMatcher::new("x,y").is_err());
        assert!(TypeMatcher::new("fd").is_err());

        assert!(XtypeMatcher::new("").is_err());
        assert!(XtypeMatcher::new("f,f").is_err());
        assert!(XtypeMatcher::new("f,").is_err());
        assert!(XtypeMatcher::new("x,y").is_err());
        assert!(XtypeMatcher::new("fd").is_err());
    }

    #[test]
    fn type_matcher_multiple_valid_types() {
        let deps = FakeDependencies::new();
        let file = get_dir_entry_for("test_data/simple", "abbbc");
        let dir = get_dir_entry_for("test_data", "simple");
        let symlink = get_dir_entry_for("test_data/links", "link-f");

        let matcher = TypeMatcher::new("f,d").unwrap();
        assert!(matcher.matches(&file, &mut deps.new_matcher_io()));
        assert!(matcher.matches(&dir, &mut deps.new_matcher_io()));
        assert!(!matcher.matches(&symlink, &mut deps.new_matcher_io()));

        let matcher = TypeMatcher::new("l,d").unwrap();
        assert!(!matcher.matches(&file, &mut deps.new_matcher_io()));
        assert!(matcher.matches(&dir, &mut deps.new_matcher_io()));
        assert!(matcher.matches(&symlink, &mut deps.new_matcher_io()));
    }

    #[cfg(unix)]
    #[test]
    fn xtype_matcher_mixed_types_with_symlinks() {
        let deps = FakeDependencies::new();

        // Regular file through symlink
        let entry = get_dir_entry_follow("test_data/links", "link-f", Follow::Always);
        let matcher = XtypeMatcher::new("f,l").unwrap();
        assert!(matcher.matches(&entry, &mut deps.new_matcher_io()));

        // Broken symlink
        let broken_entry = get_dir_entry_for("test_data/links", "link-missing");
        assert!(matcher.matches(&broken_entry, &mut deps.new_matcher_io()));

        //looping symlink
        let matcher2 = XtypeMatcher::new("l").unwrap();
        let looping_entry = get_dir_entry_for("test_data/links", "link-loop");
        assert!(matcher.matches(&looping_entry, &mut deps.new_matcher_io()));
        assert!(matcher2.matches(&looping_entry, &mut deps.new_matcher_io()));
    }
}
