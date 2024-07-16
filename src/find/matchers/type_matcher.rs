// Copyright 2017 Google Inc.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::fs::FileType;
use std::io::Write;
use std::{error::Error, io::stderr};
use walkdir::DirEntry;

#[cfg(unix)]
use std::os::unix::fs::FileTypeExt;

use super::{Matcher, MatcherIO};

/// This matcher checks the type of the file.
pub struct TypeMatcher {
    file_type_fn: fn(&FileType) -> bool,
    follow: bool,
    follow_ignore_l_option: bool,
}

impl TypeMatcher {
    pub fn new(type_string: &str, follow: bool) -> Result<Self, Box<dyn Error>> {
        #[cfg(unix)]
        let function = match type_string {
            "f" => FileType::is_file,
            "d" => FileType::is_dir,
            "l" => FileType::is_symlink,
            "b" => FileType::is_block_device,
            "c" => FileType::is_char_device,
            "p" => FileType::is_fifo, // named pipe (FIFO)
            "s" => FileType::is_socket,
            // D: door (Solaris)
            "D" => {
                return Err(From::from(format!(
                    "Type argument {type_string} not supported yet"
                )))
            }
            _ => {
                return Err(From::from(format!(
                    "Unrecognised type argument {type_string}"
                )))
            }
        };
        #[cfg(not(unix))]
        let function = match type_string {
            "f" => FileType::is_file,
            "d" => FileType::is_dir,
            "l" => FileType::is_symlink,
            _ => {
                return Err(From::from(format!(
                    "Unrecognised type argument {}",
                    type_string
                )))
            }
        };
        Ok(Self {
            file_type_fn: function,
            follow,
            // -type l will not return any results because -follow will follow symbolic links
            follow_ignore_l_option: type_string == "l" && follow,
        })
    }
}

impl Matcher for TypeMatcher {
    fn matches(&self, file_info: &DirEntry, _: &mut MatcherIO) -> bool {
        // Processing of -follow predicate:
        // 1. -type f searches not only for regular files,
        //    but also for files pointed to by symbolic links.
        // 2. -type l will not return any results because -follow will follow symbolic links,
        //    so the find command cannot find pure symbolic links.
        if self.follow_ignore_l_option {
            return false;
        }

        let file_type = if self.follow && file_info.file_type().is_symlink() {
            println!("Followed symbolic link {}", file_info.path().display());
            let path = file_info.path();
            match path.symlink_metadata() {
                Ok(file_type) => file_type.file_type(),
                Err(_) => {
                    writeln!(
                        &mut stderr(),
                        "Error getting file type for {}",
                        file_info.path().to_string_lossy()
                    )
                    .unwrap();

                    return false;
                }
            }
        } else {
            file_info.file_type()
        };

        (self.file_type_fn)(&file_type)
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

    #[cfg(windows)]
    use std::os::windows::fs::{symlink_dir, symlink_file};

    #[test]
    fn file_type_matcher() {
        [true, false].iter().for_each(|follow| {
            let file = get_dir_entry_for("test_data/simple", "abbbc");
            let dir = get_dir_entry_for("test_data", "simple");
            let deps = FakeDependencies::new();

            let matcher = TypeMatcher::new("f", *follow).unwrap();
            assert!(!matcher.matches(&dir, &mut deps.new_matcher_io()));
            assert!(matcher.matches(&file, &mut deps.new_matcher_io()));
        });
    }

    #[test]
    fn dir_type_matcher() {
        [true, false].iter().for_each(|follow| {
            let file = get_dir_entry_for("test_data/simple", "abbbc");
            let dir = get_dir_entry_for("test_data", "simple");
            let deps = FakeDependencies::new();

            let matcher = TypeMatcher::new("d", *follow).unwrap();
            assert!(matcher.matches(&dir, &mut deps.new_matcher_io()));
            assert!(!matcher.matches(&file, &mut deps.new_matcher_io()));
        });
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

        [true, false].iter().for_each(|follow| {
            let matcher = TypeMatcher::new("l", *follow).unwrap();
            assert!(!matcher.matches(&dir, &mut deps.new_matcher_io()));
            assert!(!matcher.matches(&file, &mut deps.new_matcher_io()));

            if *follow {
                // Enabling the -follow option will make this matcher always return false for type l
                assert!(!matcher.matches(&link_f, &mut deps.new_matcher_io()));
                assert!(!matcher.matches(&link_d, &mut deps.new_matcher_io()));
            } else {
                assert!(matcher.matches(&link_f, &mut deps.new_matcher_io()));
                assert!(matcher.matches(&link_d, &mut deps.new_matcher_io()));
            }
        });
    }

    #[cfg(unix)]
    #[test]
    fn unix_extra_type_matcher() {
        [true, false].iter().for_each(|follow| {
            let file = get_dir_entry_for("test_data/simple", "abbbc");
            let dir = get_dir_entry_for("test_data", "simple");
            let deps = FakeDependencies::new();

            for typ in &["b", "c", "p", "s"] {
                let matcher = TypeMatcher::new(typ, *follow).unwrap();
                assert!(!matcher.matches(&dir, &mut deps.new_matcher_io()));
                assert!(!matcher.matches(&file, &mut deps.new_matcher_io()));
            }
        });
    }

    #[test]
    fn cant_create_with_invalid_pattern() {
        let result = TypeMatcher::new("xxx", false);
        assert!(result.is_err());
    }
}
