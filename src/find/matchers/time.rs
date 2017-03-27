// Copyright 2017 Google Inc.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std;
use std::error::Error;
use std::fs::{self, Metadata};
use std::io::{stderr, Write};
use std::time::SystemTime;
use walkdir::DirEntry;

use find::matchers::{ComparableValue, Matcher, MatcherIO};

const SECONDS_PER_DAY: i64 = 60 * 60 * 24;

/// This matcher checks whether a file is newer than the file the matcher is initialized with.
pub struct NewerMatcher {
    given_modification_time: SystemTime,
}

impl NewerMatcher {
    pub fn new(path_to_file: &str) -> Result<NewerMatcher, Box<Error>> {
        let metadata = fs::metadata(path_to_file)?;
        Ok(NewerMatcher { given_modification_time: metadata.modified()? })
    }

    pub fn new_box(path_to_file: &str) -> Result<Box<Matcher>, Box<Error>> {
        Ok(Box::new(NewerMatcher::new(path_to_file)?))
    }

    /// Impementation of matches that returns a result, allowing use to use try!
    /// to deal with the errors.
    fn matches_impl(&self, file_info: &DirEntry) -> Result<bool, Box<Error>> {
        let this_time = file_info.metadata()?.modified()?;
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
}

#[derive(Clone, Copy, Debug)]
pub enum FileTimeType {
    Accessed,
    Created,
    Modified,
}

impl FileTimeType {
    fn get_file_time(self, metadata: Metadata) -> std::io::Result<SystemTime> {
        match self {
            FileTimeType::Accessed => metadata.accessed(),
            FileTimeType::Created => metadata.created(),
            FileTimeType::Modified => metadata.modified(),
        }
    }
}

/// This matcher checks whether a file's accessed|creation|modification time is
/// {less than | exactly | more than} N days old.
pub struct FileTimeMatcher {
    days: ComparableValue,
    file_time_type: FileTimeType,
}

impl Matcher for FileTimeMatcher {
    fn matches(&self, file_info: &DirEntry, matcher_io: &mut MatcherIO) -> bool {
        match self.matches_impl(file_info, matcher_io.now()) {
            Err(e) => {
                writeln!(&mut stderr(),
                         "Error getting {:?} time for {}: {}",
                         self.file_time_type,
                         file_info.path().to_string_lossy(),
                         e)
                    .unwrap();
                false
            }
            Ok(t) => t,
        }
    }
}



impl FileTimeMatcher {
    /// Impementation of matches that returns a result, allowing use to use try!
    /// to deal with the errors.
    fn matches_impl(&self, file_info: &DirEntry, now: SystemTime) -> Result<bool, Box<Error>> {
        let this_time = self.file_time_type.get_file_time(file_info.metadata()?)?;
        let mut is_negative = false;
        // durations can't be negative. So duration_since returns a duration
        // wrapped in an error if now < this_time.
        let age = match now.duration_since(this_time) {
            Ok(duration) => duration,
            Err(e) => {
                is_negative = true;
                e.duration()
            }
        };
        let age_in_seconds: i64 = age.as_secs() as i64 * if is_negative { -1 } else { 1 };
        // rust division truncates towards zero (see
        // https://github.com/rust-lang/rust/blob/master/src/libcore/ops.rs#L580 )
        // so a simple age_in_seconds / SECONDS_PER_DAY gives the wrong answer
        // for negative ages: a file whose age is 1 second in the future needs to
        // count as -1 day old, not 0.
        let age_in_days = age_in_seconds / SECONDS_PER_DAY + if is_negative { -1 } else { 0 };
        Ok(self.days.imatches(age_in_days))
    }

    pub fn new(file_time_type: FileTimeType, days: ComparableValue) -> FileTimeMatcher {
        FileTimeMatcher {
            file_time_type: file_time_type,
            days: days,
        }
    }

    pub fn new_box(file_time_type: FileTimeType, days: ComparableValue) -> Box<Matcher> {
        Box::new(FileTimeMatcher::new(file_time_type, days))
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{File, OpenOptions};
    use std::io::{Read, Write};
    use std::thread;
    use std::time::{Duration, SystemTime};
    use tempdir::TempDir;
    use walkdir::DirEntry;

    use find::matchers::{ComparableValue, Matcher};
    use find::matchers::tests::get_dir_entry_for;
    use find::tests::FakeDependencies;
    use super::*;

    #[test]
    fn newer_matcher() {
        // this file should already exist
        let old_file = get_dir_entry_for("test_data", "simple");

        let temp_dir = TempDir::new("newer_matcher").unwrap();
        let temp_dir_path = temp_dir.path().to_string_lossy();
        // this has just been created, so should be newer
        let new_file_name = "newFile";
        File::create(temp_dir.path().join(new_file_name)).expect("create temp file");

        let new_file = get_dir_entry_for(&temp_dir_path, &new_file_name);

        let matcher_for_new =
            NewerMatcher::new(&temp_dir.path().join(new_file_name).to_string_lossy()).unwrap();
        let matcher_for_old = NewerMatcher::new(&old_file.path().to_string_lossy()).unwrap();
        let deps = FakeDependencies::new();

        assert!(!matcher_for_new.matches(&old_file, &mut deps.new_matcher_io()),
                "old_file shouldn't be newer than new_dir");
        assert!(matcher_for_old.matches(&new_file, &mut deps.new_matcher_io()),
                "new_file should be newer than old_dir");
        assert!(!matcher_for_old.matches(&old_file, &mut deps.new_matcher_io()),
                "old_file shouldn't be newer than itself");
    }

    #[test]
    fn file_time_matcher() {
        // this file should already exist
        let file = get_dir_entry_for("test_data", "simple");

        let files_mtime = file.metadata().unwrap().modified().unwrap();

        let exactly_one_day_matcher = FileTimeMatcher::new(FileTimeType::Modified,
                                                           ComparableValue::EqualTo(1));
        let more_than_one_day_matcher = FileTimeMatcher::new(FileTimeType::Modified,
                                                             ComparableValue::MoreThan(1));
        let less_than_one_day_matcher = FileTimeMatcher::new(FileTimeType::Modified,
                                                             ComparableValue::LessThan(1));
        let zero_day_matcher = FileTimeMatcher::new(FileTimeType::Modified,
                                                    ComparableValue::EqualTo(0));

        // set "now" to 2 days after the file was modified.
        let mut deps = FakeDependencies::new();
        deps.set_time(files_mtime + Duration::new(2 * super::SECONDS_PER_DAY as u64, 0));
        assert!(!exactly_one_day_matcher.matches(&file, &mut deps.new_matcher_io()),
                "2 day old file shouldn't match exactly 1 day old");
        assert!(more_than_one_day_matcher.matches(&file, &mut deps.new_matcher_io()),
                "2 day old file should match more than 1 day old");
        assert!(!less_than_one_day_matcher.matches(&file, &mut deps.new_matcher_io()),
                "2 day old file shouldn't match less than 1 day old");
        assert!(!zero_day_matcher.matches(&file, &mut deps.new_matcher_io()),
                "2 day old file shouldn't match exactly 0 days old");

        // set "now" to 1 day after the file was modified.
        deps.set_time(files_mtime + Duration::new((3 * super::SECONDS_PER_DAY / 2) as u64, 0));
        assert!(exactly_one_day_matcher.matches(&file, &mut deps.new_matcher_io()),
                "1 day old file should match exactly 1 day old");
        assert!(!more_than_one_day_matcher.matches(&file, &mut deps.new_matcher_io()),
                "1 day old file shouldn't match more than 1 day old");
        assert!(!less_than_one_day_matcher.matches(&file, &mut deps.new_matcher_io()),
                "1 day old file shouldn't match less than 1 day old");
        assert!(!zero_day_matcher.matches(&file, &mut deps.new_matcher_io()),
                "1 day old file shouldn't match exactly 0 days old");

        // set "now" to exactly the same time file was modified.
        deps.set_time(files_mtime);
        assert!(!exactly_one_day_matcher.matches(&file, &mut deps.new_matcher_io()),
                "0 day old file shouldn't match exactly 1 day old");
        assert!(!more_than_one_day_matcher.matches(&file, &mut deps.new_matcher_io()),
                "0 day old file shouldn't match more than 1 day old");
        assert!(less_than_one_day_matcher.matches(&file, &mut deps.new_matcher_io()),
                "0 day old file should match less than 1 day old");
        assert!(zero_day_matcher.matches(&file, &mut deps.new_matcher_io()),
                "0 day old file should match exactly 0 days old");


        // set "now" to a second before the file was modified (e.g. the file was
        // modified after find started running
        deps.set_time(files_mtime - Duration::new(1 as u64, 0));
        assert!(!exactly_one_day_matcher.matches(&file, &mut deps.new_matcher_io()),
                "future-modified file shouldn'1 match exactly 1 day old");
        assert!(!more_than_one_day_matcher.matches(&file, &mut deps.new_matcher_io()),
                "future-modified file shouldn't match more than 1 day old");
        assert!(less_than_one_day_matcher.matches(&file, &mut deps.new_matcher_io()),
                "future-modified file should match less than 1 day old");
        assert!(!zero_day_matcher.matches(&file, &mut deps.new_matcher_io()),
                "future-modified file shouldn't match exactly 0 days old");

    }

    #[test]
    fn file_time_matcher_modified_created_accessed() {

        let temp_dir = TempDir::new("file_time_matcher_modified_created_accessed").unwrap();

        // No easy way to independently set file times. So create it - setting creation time
        let foo_path = temp_dir.path().join("foo");
        {
            File::create(&foo_path).expect("create temp file");
        }

        thread::sleep(Duration::from_secs(2));
        // read the file - potentially changing accessed time
        let mut buffer = [0; 10];
        {
            let mut f = File::open(&foo_path).expect("open temp file");
            let _ = f.read(&mut buffer);
        }

        thread::sleep(Duration::from_secs(2));
        // write to the file - changing the modifiation and potentially the accessed time
        let mut buffer = [0; 10];
        {
            let mut f =
                OpenOptions::new().read(true).write(true).open(&foo_path).expect("open temp file");
            let _ = f.write(&mut buffer);
        }


        thread::sleep(Duration::from_secs(2));
        // read the file agaion - potentially changing accessed time
        {
            let mut f = File::open(&foo_path).expect("open temp file");
            let _ = f.read(&mut buffer);
        }

        // OK our modification time and creation time should definitely be different
        // and depending on our platform and file system, our accessed time migh be
        // different too.

        let file_info = get_dir_entry_for(&temp_dir.path().to_string_lossy(), "foo");
        let metadata = file_info.metadata().unwrap();

        // metadata can return errors like StringError("creation time is not available on this platform currently")
        // so skip tests that won't pass due to shortcomings in std:;fs.
        if let Ok(accessed_time) = metadata.accessed() {
            test_matcher_for_file_time_type(&file_info, accessed_time, FileTimeType::Accessed);
        }

        if let Ok(creation_time) = metadata.created() {
            test_matcher_for_file_time_type(&file_info, creation_time, FileTimeType::Created);
        }

        if let Ok(modified_time) = metadata.modified() {
            test_matcher_for_file_time_type(&file_info, modified_time, FileTimeType::Modified);
        }
    }

    /// helper function for file_time_matcher_modified_created_accessed
    fn test_matcher_for_file_time_type(file_info: &DirEntry,
                                       file_time: SystemTime,
                                       file_time_type: FileTimeType) {
        {
            let matcher = FileTimeMatcher::new(file_time_type, ComparableValue::EqualTo(0));

            let mut deps = FakeDependencies::new();
            deps.set_time(file_time);
            assert!(matcher.matches(&file_info, &mut deps.new_matcher_io()),
                    "{:?} time matcher should match",
                    file_time_type);

            deps.set_time(file_time - Duration::from_secs(1));
            assert!(!matcher.matches(&file_info, &mut deps.new_matcher_io()),
                    "{:?} time matcher shouldn't match a second before",
                    file_time_type);
        }
    }
}
