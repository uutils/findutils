// Copyright 2017 Google Inc.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::error::Error;
use std::fs::{self, Metadata};
use std::io::{stderr, Write};
use std::time::{SystemTime, UNIX_EPOCH};
use walkdir::DirEntry;

use super::{ComparableValue, Matcher, MatcherIO};

const SECONDS_PER_DAY: i64 = 60 * 60 * 24;

/// This matcher checks whether a file is newer than the file the matcher is initialized with.
pub struct NewerMatcher {
    given_modification_time: SystemTime,
}

impl NewerMatcher {
    pub fn new(path_to_file: &str) -> Result<Self, Box<dyn Error>> {
        let metadata = fs::metadata(path_to_file)?;
        Ok(Self {
            given_modification_time: metadata.modified()?,
        })
    }

    /// Implementation of matches that returns a result, allowing use to use try!
    /// to deal with the errors.
    fn matches_impl(&self, file_info: &DirEntry) -> Result<bool, Box<dyn Error>> {
        let this_time = file_info.metadata()?.modified()?;
        // duration_since returns an Ok duration if this_time <= given_modification_time
        // and returns an Err (with a duration) otherwise. So if this_time >
        // given_modification_time (in which case we want to return true) then
        // duration_since will return an error.
        Ok(self
            .given_modification_time
            .duration_since(this_time)
            .is_err())
    }
}

impl Matcher for NewerMatcher {
    fn matches(&self, file_info: &DirEntry, _: &mut MatcherIO) -> bool {
        match self.matches_impl(file_info) {
            Err(e) => {
                writeln!(
                    &mut stderr(),
                    "Error getting modification time for {}: {}",
                    file_info.path().to_string_lossy(),
                    e
                )
                .unwrap();
                false
            }
            Ok(t) => t,
        }
    }
}

/// `-newerXY` option.
/// a is meaning Accessed time
/// B is meaning Birthed time
/// c is meaning Changed time
/// m is meaning Modified time
/// It should be noted that not every file system supports birthed time.
#[derive(Clone, Copy, Debug)]
pub enum NewerOptionType {
    Accessed,
    Birthed,
    Changed,
    Modified,
}

impl NewerOptionType {
    pub fn from_str(option: &str) -> Self {
        match option {
            "a" => NewerOptionType::Accessed,
            "B" => NewerOptionType::Birthed,
            "c" => NewerOptionType::Changed,
            _ => NewerOptionType::Modified,
        }
    }

    fn get_file_time(self, metadata: Metadata) -> std::io::Result<SystemTime> {
        match self {
            NewerOptionType::Accessed => metadata.accessed(),
            NewerOptionType::Birthed => metadata.created(),
            // metadata.ctime() only impl in MetadataExt
            NewerOptionType::Changed => metadata.accessed(),
            NewerOptionType::Modified => metadata.modified(),
        }
    }
}

/// This matcher checks whether the file is newer than the file time of any combination of
/// two comparison types from the target file's `NewerOptionType`.
pub struct NewerOptionMatcher {
    x_option: NewerOptionType,
    y_option: NewerOptionType,
    given_modification_time: SystemTime,
}

impl NewerOptionMatcher {
    pub fn new(
        x_option: String,
        y_option: String,
        path_to_file: &str,
    ) -> Result<Self, Box<dyn Error>> {
        let metadata = fs::metadata(path_to_file)?;
        let x_option = NewerOptionType::from_str(x_option.as_str());
        let y_option = NewerOptionType::from_str(y_option.as_str());
        Ok(Self {
            x_option,
            y_option,
            given_modification_time: metadata.modified()?,
        })
    }

    fn matches_impl(&self, file_info: &DirEntry) -> Result<bool, Box<dyn Error>> {
        let x_option_time = self.x_option.get_file_time(file_info.metadata()?)?;
        let y_option_time = self.y_option.get_file_time(file_info.metadata()?)?;

        Ok(self
            .given_modification_time
            .duration_since(x_option_time)
            .is_err()
            && self
                .given_modification_time
                .duration_since(y_option_time)
                .is_err())
    }
}

impl Matcher for NewerOptionMatcher {
    fn matches(&self, file_info: &DirEntry, _: &mut MatcherIO) -> bool {
        match self.matches_impl(file_info) {
            Err(e) => {
                writeln!(
                    &mut stderr(),
                    "Error getting {:?} and {:?} time for {}: {}",
                    self.x_option,
                    self.y_option,
                    file_info.path().to_string_lossy(),
                    e
                )
                .unwrap();
                false
            }
            Ok(t) => t,
        }
    }
}

/// This matcher checks whether files's accessed|creation|modification time is
/// newer than the given times.
pub struct NewerTimeMatcher {
    time: i64,
    newer_time_type: NewerOptionType,
}

impl NewerTimeMatcher {
    pub fn new(newer_time_type: NewerOptionType, time: i64) -> Self {
        Self {
            time,
            newer_time_type,
        }
    }

    fn matches_impl(&self, file_info: &DirEntry) -> Result<bool, Box<dyn Error>> {
        let this_time = self.newer_time_type.get_file_time(file_info.metadata()?)?;
        let timestamp = this_time
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|e| e.duration());

        // timestamp.as_millis() return u128 but time is i64
        // This may leave memory implications. :(
        Ok(self.time
            <= timestamp
                .as_millis()
                .try_into()
                .expect("timestamp memory implications"))
    }
}

impl Matcher for NewerTimeMatcher {
    fn matches(&self, file_info: &DirEntry, _: &mut MatcherIO) -> bool {
        match self.matches_impl(file_info) {
            Err(e) => {
                writeln!(
                    &mut stderr(),
                    "Error getting {:?} time for {}: {}",
                    self.newer_time_type,
                    file_info.path().to_string_lossy(),
                    e
                )
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
    today_start: bool,
}

impl Matcher for FileTimeMatcher {
    fn matches(&self, file_info: &DirEntry, matcher_io: &mut MatcherIO) -> bool {
        match self.matches_impl(file_info, matcher_io.now()) {
            Err(e) => {
                writeln!(
                    &mut stderr(),
                    "Error getting {:?} time for {}: {}",
                    self.file_time_type,
                    file_info.path().to_string_lossy(),
                    e
                )
                .unwrap();
                false
            }
            Ok(t) => t,
        }
    }
}

impl FileTimeMatcher {
    /// Implementation of matches that returns a result, allowing use to use try!
    /// to deal with the errors.
    fn matches_impl(&self, file_info: &DirEntry, now: SystemTime) -> Result<bool, Box<dyn Error>> {
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

    pub fn new(file_time_type: FileTimeType, days: ComparableValue, today_start: bool) -> Self {
        Self {
            days,
            file_time_type,
            today_start,
        }
    }
}

pub struct FileAgeRangeMatcher {
    minutes: ComparableValue,
    file_time_type: FileTimeType,
    today_start: bool,
}

impl Matcher for FileAgeRangeMatcher {
    fn matches(&self, file_info: &DirEntry, matcher_io: &mut MatcherIO) -> bool {
        match self.matches_impl(file_info, matcher_io.now()) {
            Err(e) => {
                writeln!(
                    &mut stderr(),
                    "Error getting {:?} time for {}: {}",
                    self.file_time_type,
                    file_info.path().to_string_lossy(),
                    e
                )
                .unwrap();
                false
            }
            Ok(t) => t,
        }
    }
}

impl FileAgeRangeMatcher {
    fn matches_impl(&self, file_info: &DirEntry, now: SystemTime) -> Result<bool, Box<dyn Error>> {
        let this_time = self.file_time_type.get_file_time(file_info.metadata()?)?;
        let mut is_negative = false;
        let age = match now.duration_since(this_time) {
            Ok(duration) => duration,
            Err(e) => {
                is_negative = true;
                e.duration()
            }
        };
        let age_in_seconds: i64 = age.as_secs() as i64 * if is_negative { -1 } else { 1 };
        let age_in_minutes = age_in_seconds / 60 + if is_negative { -1 } else { 0 };
        Ok(self.minutes.imatches(age_in_minutes))
    }

    pub fn new(file_time_type: FileTimeType, minutes: ComparableValue, today_start: bool) -> Self {
        Self {
            minutes,
            file_time_type,
            today_start,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::fs::{File, OpenOptions};
    use std::io::Read;
    use std::thread;
    use std::time::Duration;
    use tempfile::Builder;

    use super::*;
    use crate::find::matchers::tests::get_dir_entry_for;
    use crate::find::tests::FakeDependencies;

    #[test]
    fn newer_matcher() {
        // this file should already exist
        let old_file = get_dir_entry_for("test_data", "simple");

        let temp_dir = Builder::new().prefix("example").tempdir().unwrap();
        let temp_dir_path = temp_dir.path().to_string_lossy();
        // this has just been created, so should be newer
        let new_file_name = "newFile";
        File::create(temp_dir.path().join(new_file_name)).expect("create temp file");

        let new_file = get_dir_entry_for(&temp_dir_path, new_file_name);

        let matcher_for_new =
            NewerMatcher::new(&temp_dir.path().join(new_file_name).to_string_lossy()).unwrap();
        let matcher_for_old = NewerMatcher::new(&old_file.path().to_string_lossy()).unwrap();
        let deps = FakeDependencies::new();

        assert!(
            !matcher_for_new.matches(&old_file, &mut deps.new_matcher_io()),
            "old_file shouldn't be newer than new_dir"
        );
        assert!(
            matcher_for_old.matches(&new_file, &mut deps.new_matcher_io()),
            "new_file should be newer than old_dir"
        );
        assert!(
            !matcher_for_old.matches(&old_file, &mut deps.new_matcher_io()),
            "old_file shouldn't be newer than itself"
        );
    }

    #[test]
    fn file_time_matcher() {
        // this file should already exist
        let file = get_dir_entry_for("test_data", "simple");

        let files_mtime = file.metadata().unwrap().modified().unwrap();

        let exactly_one_day_matcher =
            FileTimeMatcher::new(FileTimeType::Modified, ComparableValue::EqualTo(1), false);
        let more_than_one_day_matcher =
            FileTimeMatcher::new(FileTimeType::Modified, ComparableValue::MoreThan(1), false);
        let less_than_one_day_matcher =
            FileTimeMatcher::new(FileTimeType::Modified, ComparableValue::LessThan(1), false);
        let zero_day_matcher =
            FileTimeMatcher::new(FileTimeType::Modified, ComparableValue::EqualTo(0), false);

        // set "now" to 2 days after the file was modified.
        let mut deps = FakeDependencies::new();
        deps.set_time(files_mtime + Duration::new(2 * SECONDS_PER_DAY as u64, 0));
        assert!(
            !exactly_one_day_matcher.matches(&file, &mut deps.new_matcher_io()),
            "2 day old file shouldn't match exactly 1 day old"
        );
        assert!(
            more_than_one_day_matcher.matches(&file, &mut deps.new_matcher_io()),
            "2 day old file should match more than 1 day old"
        );
        assert!(
            !less_than_one_day_matcher.matches(&file, &mut deps.new_matcher_io()),
            "2 day old file shouldn't match less than 1 day old"
        );
        assert!(
            !zero_day_matcher.matches(&file, &mut deps.new_matcher_io()),
            "2 day old file shouldn't match exactly 0 days old"
        );

        // set "now" to 1 day after the file was modified.
        deps.set_time(files_mtime + Duration::new((3 * SECONDS_PER_DAY / 2) as u64, 0));
        assert!(
            exactly_one_day_matcher.matches(&file, &mut deps.new_matcher_io()),
            "1 day old file should match exactly 1 day old"
        );
        assert!(
            !more_than_one_day_matcher.matches(&file, &mut deps.new_matcher_io()),
            "1 day old file shouldn't match more than 1 day old"
        );
        assert!(
            !less_than_one_day_matcher.matches(&file, &mut deps.new_matcher_io()),
            "1 day old file shouldn't match less than 1 day old"
        );
        assert!(
            !zero_day_matcher.matches(&file, &mut deps.new_matcher_io()),
            "1 day old file shouldn't match exactly 0 days old"
        );

        // set "now" to exactly the same time file was modified.
        deps.set_time(files_mtime);
        assert!(
            !exactly_one_day_matcher.matches(&file, &mut deps.new_matcher_io()),
            "0 day old file shouldn't match exactly 1 day old"
        );
        assert!(
            !more_than_one_day_matcher.matches(&file, &mut deps.new_matcher_io()),
            "0 day old file shouldn't match more than 1 day old"
        );
        assert!(
            less_than_one_day_matcher.matches(&file, &mut deps.new_matcher_io()),
            "0 day old file should match less than 1 day old"
        );
        assert!(
            zero_day_matcher.matches(&file, &mut deps.new_matcher_io()),
            "0 day old file should match exactly 0 days old"
        );

        // set "now" to a second before the file was modified (e.g. the file was
        // modified after find started running
        deps.set_time(files_mtime - Duration::new(1_u64, 0));
        assert!(
            !exactly_one_day_matcher.matches(&file, &mut deps.new_matcher_io()),
            "future-modified file shouldn't match exactly 1 day old"
        );
        assert!(
            !more_than_one_day_matcher.matches(&file, &mut deps.new_matcher_io()),
            "future-modified file shouldn't match more than 1 day old"
        );
        assert!(
            less_than_one_day_matcher.matches(&file, &mut deps.new_matcher_io()),
            "future-modified file should match less than 1 day old"
        );
        assert!(
            !zero_day_matcher.matches(&file, &mut deps.new_matcher_io()),
            "future-modified file shouldn't match exactly 0 days old"
        );
    }

    #[test]
    fn file_time_matcher_modified_created_accessed() {
        let temp_dir = Builder::new()
            .prefix("file_time_matcher_modified_created_accessed")
            .tempdir()
            .unwrap();

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
        // write to the file - changing the modification and potentially the accessed time
        let mut buffer = [0; 10];
        {
            let mut f = OpenOptions::new()
                .read(true)
                .write(true)
                .open(&foo_path)
                .expect("open temp file");
            let _ = f.write(&buffer);
        }

        thread::sleep(Duration::from_secs(2));
        // read the file again - potentially changing accessed time
        {
            let mut f = File::open(&foo_path).expect("open temp file");
            let _ = f.read(&mut buffer);
        }

        // OK our modification time and creation time should definitely be different
        // and depending on our platform and file system, our accessed time might be
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

    /// helper function for `file_time_matcher_modified_created_accessed`
    fn test_matcher_for_file_time_type(
        file_info: &DirEntry,
        file_time: SystemTime,
        file_time_type: FileTimeType,
    ) {
        {
            let matcher = FileTimeMatcher::new(file_time_type, ComparableValue::EqualTo(0), false);

            let mut deps = FakeDependencies::new();
            deps.set_time(file_time);
            assert!(
                matcher.matches(file_info, &mut deps.new_matcher_io()),
                "{file_time_type:?} time matcher should match"
            );

            deps.set_time(file_time - Duration::from_secs(1));
            assert!(
                !matcher.matches(file_info, &mut deps.new_matcher_io()),
                "{file_time_type:?} time matcher shouldn't match a second before"
            );
        }
    }

    #[test]
    fn newer_option_matcher() {
        #[cfg(target_os = "linux")]
        let options = ["a", "c", "m"];
        #[cfg(not(target_os = "linux"))]
        let options = ["a", "B", "c", "m"];

        for x_option in options {
            for y_option in options {
                let temp_dir = Builder::new().prefix("example").tempdir().unwrap();
                let temp_dir_path = temp_dir.path().to_string_lossy();
                let new_file_name = "newFile";
                // this has just been created, so should be newer
                File::create(temp_dir.path().join(new_file_name)).expect("create temp file");
                let new_file = get_dir_entry_for(&temp_dir_path, new_file_name);
                // this file should already exist
                let old_file = get_dir_entry_for("test_data", "simple");
                let deps = FakeDependencies::new();
                let matcher = NewerOptionMatcher::new(
                    x_option.to_string(),
                    y_option.to_string(),
                    &old_file.path().to_string_lossy(),
                );

                assert!(
                    matcher
                        .unwrap()
                        .matches(&new_file, &mut deps.new_matcher_io()),
                    "new_file should be newer than old_dir"
                );

                // After the file is deleted, DirEntry will point to an empty file location,
                // thus causing the Matcher to generate an IO error after matching.
                //
                // Note: This test is nondeterministic on Windows,
                // because fs::remove_file may not actually remove the file from
                // the file system even if it returns Ok.
                // Therefore, this test will only be performed on Linux/Unix.
                let _ = fs::remove_file(&*new_file.path().to_string_lossy());

                #[cfg(unix)]
                {
                    let matcher = NewerOptionMatcher::new(
                        x_option.to_string(),
                        y_option.to_string(),
                        &old_file.path().to_string_lossy(),
                    );
                    assert!(
                        !matcher
                            .unwrap()
                            .matches(&new_file, &mut deps.new_matcher_io()),
                        "The correct situation is that the file reading here cannot be successful."
                    );
                }
            }
        }
    }

    #[test]
    fn newer_time_matcher() {
        let deps = FakeDependencies::new();
        let time = deps
            .new_matcher_io()
            .now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis()
            .try_into()
            .unwrap();

        let created_matcher = NewerTimeMatcher::new(NewerOptionType::Birthed, time);

        thread::sleep(Duration::from_millis(100));
        let temp_dir = Builder::new()
            .prefix("newer_time_matcher")
            .tempdir()
            .unwrap();
        // No easy way to independently set file times. So create it - setting creation time
        let foo_path = temp_dir.path().join("foo");
        // after "time" created a file
        let _ = File::create(&foo_path).expect("create temp file");
        // so this file created time should after "time"
        let file_info = get_dir_entry_for(&temp_dir.path().to_string_lossy(), "foo");
        assert!(
            created_matcher.matches(&file_info, &mut deps.new_matcher_io()),
            "file created time should after 'time'"
        );

        // accessed time test
        let accessed_matcher = NewerTimeMatcher::new(NewerOptionType::Accessed, time);
        let mut buffer = [0; 10];
        {
            let mut file = File::open(&foo_path).expect("open temp file");
            let _ = file.read(&mut buffer);
        }
        assert!(
            accessed_matcher.matches(&file_info, &mut deps.new_matcher_io()),
            "file accessed time should after 'time'"
        );

        // modified time test
        let modified_matcher = NewerTimeMatcher::new(NewerOptionType::Modified, time);
        let buffer = [0; 10];
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&foo_path)
            .expect("open temp file");
        let _ = file.write(&buffer);

        assert!(
            modified_matcher.matches(&file_info, &mut deps.new_matcher_io()),
            "file modified time should after 'time'"
        );

        let inode_changed_matcher = NewerTimeMatcher::new(NewerOptionType::Changed, time);
        // Steps to change inode:
        // 1. Copy and rename the file
        // 2. Delete the old file
        // 3. Change the new file name to the old file name
        let _ = File::create(temp_dir.path().join("inode_test_file")).expect("create temp file");
        let _ = fs::copy("inode_test_file", "new_inode_test_file");
        let _ = fs::remove_file("inode_test_file");
        let _ = fs::rename("new_inode_test_file", "inode_test_file");
        let file_info = get_dir_entry_for(&temp_dir.path().to_string_lossy(), "inode_test_file");
        assert!(
            inode_changed_matcher.matches(&file_info, &mut deps.new_matcher_io()),
            "file inode changed time should after 'std_time'"
        );

        // After the file is deleted, DirEntry will point to an empty file location,
        // thus causing the Matcher to generate an IO error after matching.
        //
        // Note: This test is nondeterministic on Windows,
        // because fs::remove_file may not actually remove the file from
        // the file system even if it returns Ok.
        // Therefore, this test will only be performed on Linux/Unix.
        let _ = fs::remove_file(&*file_info.path().to_string_lossy());

        #[cfg(unix)]
        {
            let matchers = [
                &created_matcher,
                &accessed_matcher,
                &modified_matcher,
                &inode_changed_matcher,
            ];

            for matcher in &matchers {
                assert!(
                    !matcher.matches(&file_info, &mut deps.new_matcher_io()),
                    "The correct situation is that the file reading here cannot be successful."
                );
            }
        }
    }

    #[test]
    fn file_age_range_matcher() {
        let temp_dir = Builder::new().prefix("example").tempdir().unwrap();
        let temp_dir_path = temp_dir.path().to_string_lossy();
        let new_file_name = "newFile";
        // this has just been created, so should be newer
        File::create(temp_dir.path().join(new_file_name)).expect("create temp file");
        let new_file = get_dir_entry_for(&temp_dir_path, new_file_name);

        // more test
        // mocks:
        // - find test_data/simple -amin +1
        // - find test_data/simple -cmin +1
        // - find test_data/simple -mmin +1
        // Means to find files accessed / modified more than 1 minute ago.
        [
            FileTimeType::Accessed,
            FileTimeType::Created,
            FileTimeType::Modified,
        ]
        .iter()
        .for_each(|time_type| {
            let more_matcher =
                FileAgeRangeMatcher::new(*time_type, ComparableValue::MoreThan(1), false);
            assert!(
                !more_matcher.matches(&new_file, &mut FakeDependencies::new().new_matcher_io()),
                "{}",
                format!(
                    "more minutes old file should match more than 1 minute old in {} test.",
                    match *time_type {
                        FileTimeType::Accessed => "accessed",
                        FileTimeType::Created => "created",
                        FileTimeType::Modified => "modified",
                    }
                )
            );
        });

        // less test
        // mocks:
        // - find test_data/simple -amin -1
        // - find test_data/simple -cmin -1
        // - find test_data/simple -mmin -1
        // Means to find files accessed / modified less than 1 minute ago.
        [
            FileTimeType::Accessed,
            FileTimeType::Created,
            FileTimeType::Modified,
        ]
        .iter()
        .for_each(|time_type| {
            let less_matcher =
                FileAgeRangeMatcher::new(*time_type, ComparableValue::LessThan(1), false);
            assert!(
                less_matcher.matches(&new_file, &mut FakeDependencies::new().new_matcher_io()),
                "{}",
                format!(
                    "less minutes old file should not match less than 1 minute old in {} test.",
                    match *time_type {
                        FileTimeType::Accessed => "accessed",
                        FileTimeType::Created => "created",
                        FileTimeType::Modified => "modified",
                    }
                )
            );
        });

        // catch file error
        let _ = fs::remove_file(&*new_file.path().to_string_lossy());
        let matcher =
            FileAgeRangeMatcher::new(FileTimeType::Modified, ComparableValue::MoreThan(1), false);
        assert!(
            !matcher.matches(&new_file, &mut FakeDependencies::new().new_matcher_io()),
            "The correct situation is that the file reading here cannot be successful."
        );
    }
}
