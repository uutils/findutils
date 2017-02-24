/// ! This module contains matchers that compare date/times associated with files.
/// ! The code is more complicated than I'd like because it works with SystemTime,
/// ! which are opaque objects, and Durations, which can't be negative.

use std::error::Error;
use std::fs::File;
use std::io::{stderr, Write};
use std::time::SystemTime;
use walkdir::DirEntry;

use find::matchers::{Matcher, MatcherIO};

/// This matcher checks whether a file is newer than the file it is initialized with.
pub struct NewerMatcher {
    given_modification_time: SystemTime,
}

impl NewerMatcher {
    pub fn new(path_to_file: &str) -> Result<NewerMatcher, Box<Error>> {
        let f = try!(File::open(path_to_file));
        let metadata = try!(f.metadata());
        Ok(NewerMatcher { given_modification_time: try!(metadata.modified()) })
    }

    pub fn new_box(path_to_file: &str) -> Result<Box<Matcher>, Box<Error>> {
        Ok(Box::new(try!(NewerMatcher::new(path_to_file))))
    }

    /// Impementation of matches that returns a result, allowing use to use try!
    /// to deal with the errors.
    fn matches_impl(&self, file_info: &DirEntry) -> Result<bool, Box<Error>> {
        let this_time = try!(try!(file_info.metadata()).modified());
        // duration_since returns an Ok duration if this_time <= given_modification_time
        // and returns an Err (with a duration) otherwise. So if this_time >
        // given_modification_time (in which case we want to return true) then
        // duration_since will return an error.
        Ok(self.given_modification_time.duration_since(this_time).is_err())
    }
}

impl Matcher for NewerMatcher {
    fn matches(&self, file_info: &DirEntry, _: &mut MatcherIO) -> bool {
        match self.matches_impl(file_info) {
            Err(e) => {
                writeln!(&mut stderr(),
                         "Error getting modification time for {}: {}",
                         file_info.path().to_string_lossy(),
                         e)
                    .unwrap();
                false
            }
            Ok(t) => t,
        }
    }

    fn has_side_effects(&self) -> bool {
        false
    }
}
#[cfg(test)]

mod tests {
    use std::fs::File;
    use tempdir::TempDir;

    use find::matchers::Matcher;
    use find::matchers::tests::get_dir_entry_for;
    use find::tests::FakeDependencies;
    use super::*;

    #[test]
    fn newer_matcher() {
        // this file should already exist
        let old_file = get_dir_entry_for("test_data", "simple");

        // this has just been created, so should be newer
        let temp_dir = TempDir::new("newer_matcher").unwrap();
        let temp_dir_path = temp_dir.path().to_string_lossy();
        let new_file_name = "newFile";
        File::create(temp_dir.path().join(new_file_name)).expect("create temp file");

        let new_file = get_dir_entry_for(&temp_dir_path, &new_file_name);

        let matcher_for_new =
            NewerMatcher::new(&temp_dir.path().join(new_file_name).to_string_lossy()).unwrap();
        let matcher_for_old = NewerMatcher::new(&old_file.path().to_string_lossy()).unwrap();
        let deps = FakeDependencies::new();

        // old_file isn't newer than new_dir
        assert!(!matcher_for_new.matches(&old_file, &mut deps.new_side_effects()));
        // old_file isn't newer than new_dir
        assert!(matcher_for_old.matches(&new_file, &mut deps.new_side_effects()));
        // old_file isn't newer than itself
        assert!(!matcher_for_old.matches(&old_file, &mut deps.new_side_effects()));
    }
    //    #[test]
    //    fn dir_type_matcher() {
    //        let file = get_dir_entry_for("test_data/simple", "abbbc");
    //        let dir = get_dir_entry_for("test_data", "simple");
    //        let deps = FakeDependencies::new();
    //
    //        let matcher = TypeMatcher::new(&"d".to_string()).unwrap();
    //        assert!(matcher.matches(&dir, &mut deps.new_side_effects()));
    //        assert!(!matcher.matches(&file, &mut deps.new_side_effects()));
    //    }
    //
    //    #[test]
    //    fn cant_create_with_invalid_pattern() {
    //        let result = TypeMatcher::new(&"xxx".to_string());
    //        assert!(result.is_err());
    //    }

}
