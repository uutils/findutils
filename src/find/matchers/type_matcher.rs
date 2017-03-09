// Copyright 2017 Google Inc.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::error::Error;
use std::fs::FileType;
use walkdir::DirEntry;

use find::matchers::{Matcher, MatcherIO};

/// This matcher checks the type of the file.
pub struct TypeMatcher {
    file_type_fn: fn(&FileType) -> bool,
}

impl TypeMatcher {
    pub fn new(type_string: &str) -> Result<TypeMatcher, Box<Error>> {
        let function = match type_string {
            "f" => FileType::is_file,
            "d" => FileType::is_dir,
            "b" | "c" | "p" | "l" | "s" | "D" => {
                return Err(From::from(format!("Type argument {} not supported yet", type_string)))
            }
            _ => return Err(From::from(format!("Unrecognised type argument {}", type_string))),
        };
        Ok(TypeMatcher { file_type_fn: function })
    }

    pub fn new_box(type_string: &str) -> Result<Box<Matcher>, Box<Error>> {
        Ok(Box::new(TypeMatcher::new(type_string)?))
    }
}

impl Matcher for TypeMatcher {
    fn matches(&self, file_info: &DirEntry, _: &mut MatcherIO) -> bool {
        (self.file_type_fn)(&file_info.file_type())
    }
}
#[cfg(test)]

mod tests {
    use find::matchers::Matcher;
    use find::matchers::tests::get_dir_entry_for;
    use find::tests::FakeDependencies;
    use super::*;

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

    #[test]
    fn cant_create_with_invalid_pattern() {
        let result = TypeMatcher::new(&"xxx".to_string());
        assert!(result.is_err());
    }

}
