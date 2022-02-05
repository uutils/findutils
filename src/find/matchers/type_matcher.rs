// Copyright 2017 Google Inc.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::error::Error;
use std::fs::FileType;
use walkdir::DirEntry;

#[cfg(unix)]
use std::os::unix::fs::FileTypeExt;

use super::{Matcher, MatcherIO};

/// This matcher checks the type of the file.
pub struct TypeMatcher {
    file_type_fn: fn(&FileType) -> bool,
}

impl TypeMatcher {
    pub fn new(type_string: &str) -> Result<Self, Box<dyn Error>> {
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
                    "Type argument {} not supported yet",
                    type_string
                )))
            }
            _ => {
                return Err(From::from(format!(
                    "Unrecognised type argument {}",
                    type_string
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
        })
    }

    pub fn new_box(type_string: &str) -> Result<Box<dyn Matcher>, Box<dyn Error>> {
        Ok(Box::new(Self::new(type_string)?))
    }
}

impl Matcher for TypeMatcher {
    fn matches(&self, file_info: &DirEntry, _: &mut MatcherIO) -> bool {
        (self.file_type_fn)(&file_info.file_type())
    }
}
#[cfg(test)]

mod tests {
    use super::*;
    use crate::find::matchers::tests::get_dir_entry_for;
    use crate::find::matchers::Matcher;
    use crate::find::tests::FakeDependencies;
    use std::io::ErrorKind;

    #[cfg(unix)]
    use std::os::unix::fs::symlink;

    #[cfg(windows)]
    use std::os::windows::fs::{symlink_dir, symlink_file};

    #[test]
    fn file_type_matcher() {
        let file = get_dir_entry_for("test_data/simple", "abbbc");
        let dir = get_dir_entry_for("test_data", "simple");
        let deps = FakeDependencies::new();

        let matcher = TypeMatcher::new(&"f".to_string()).unwrap();
        assert!(!matcher.matches(&dir, &mut deps.new_matcher_io()));
        assert!(matcher.matches(&file, &mut deps.new_matcher_io()));
    }

    #[test]
    fn dir_type_matcher() {
        let file = get_dir_entry_for("test_data/simple", "abbbc");
        let dir = get_dir_entry_for("test_data", "simple");
        let deps = FakeDependencies::new();

        let matcher = TypeMatcher::new(&"d".to_string()).unwrap();
        assert!(matcher.matches(&dir, &mut deps.new_matcher_io()));
        assert!(!matcher.matches(&file, &mut deps.new_matcher_io()));
    }

    // git does not translate links (in test_data) to Windows links
    // so we have to create links in test
    #[test]
    fn link_type_matcher() {
        #[cfg(unix)]
        let _ = {
            if let Err(e) = symlink("abbbc", "test_data/links/link-f") {
                if e.kind() != ErrorKind::AlreadyExists {
                    panic!("Failed to create sym link: {:?}", e);
                }
            }
            if let Err(e) = symlink("subdir", "test_data/links/link-d") {
                if e.kind() != ErrorKind::AlreadyExists {
                    panic!("Failed to create sym link: {:?}", e);
                }
            }
        };
        #[cfg(windows)]
        let _ = {
            if let Err(e) = symlink_file("abbbc", "test_data/links/link-f") {
                if e.kind() != ErrorKind::AlreadyExists {
                    panic!("Failed to create sym link: {:?}", e);
                }
            }
            if let Err(e) = symlink_dir("subdir", "test_data/links/link-d") {
                if e.kind() != ErrorKind::AlreadyExists {
                    panic!("Failed to create sym link: {:?}", e);
                }
            }
        };

        let link_f = get_dir_entry_for("test_data/links", "link-f");
        let link_d = get_dir_entry_for("test_data/links", "link-d");
        let file = get_dir_entry_for("test_data/links", "abbbc");
        let dir = get_dir_entry_for("test_data", "links");
        let deps = FakeDependencies::new();

        let matcher = TypeMatcher::new(&"l".to_string()).unwrap();
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

        for typ in ["b", "c", "p", "s"].iter() {
            let matcher = TypeMatcher::new(&typ.to_string()).unwrap();
            assert!(!matcher.matches(&dir, &mut deps.new_matcher_io()));
            assert!(!matcher.matches(&file, &mut deps.new_matcher_io()));
        }
    }

    #[test]
    fn cant_create_with_invalid_pattern() {
        let result = TypeMatcher::new(&"xxx".to_string());
        assert!(result.is_err());
    }
}
