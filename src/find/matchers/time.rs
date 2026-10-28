// Copyright 2017 Google Inc.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::error::Error;
use std::fs::{self, Metadata};
use std::io::{stderr, Write};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Local, Timelike};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

use super::{ComparableValue, Follow, Matcher, MatcherIO, WalkEntry};

const SECONDS_PER_DAY: i64 = 60 * 60 * 24;

fn get_time(matcher_io: &mut MatcherIO, today_start: bool) -> SystemTime {
    if today_start {
        // the time at 00:00:00 of today
        let duration_since_unix_epoch = matcher_io.now().duration_since(UNIX_EPOCH).unwrap();
        let seconds_since_unix_epoch = duration_since_unix_epoch.as_secs();
        let utc_time = DateTime::from_timestamp(seconds_since_unix_epoch as i64, 0).unwrap();
        let local_time = utc_time.with_timezone(&Local);
        let seconds_since_last_midnight = local_time.num_seconds_from_midnight();
        let local_midnight_seconds = local_time.timestamp() - seconds_since_last_midnight as i64;

        UNIX_EPOCH + Duration::from_secs(local_midnight_seconds as u64)
    } else {
        matcher_io.now()
    }
}

/// This matcher checks whether a file is newer than the file the matcher is initialized with.
pub struct NewerMatcher {
    given_modification_time: SystemTime,
}

impl NewerMatcher {
    pub fn new(path_to_file: &str, follow: Follow) -> Result<Self, Box<dyn Error>> {
        let metadata = follow.root_metadata(path_to_file)?;
        Ok(Self {
            given_modification_time: metadata.modified()?,
        })
    }

    /// Implementation of matches that returns a result, allowing use to use try!
    /// to deal with the errors.
    fn matches_impl(&self, file_info: &WalkEntry) -> Result<bool, Box<dyn Error>> {
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
    fn matches(&self, file_info: &WalkEntry, _: &mut MatcherIO) -> bool {
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
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(option: &str) -> Self {
        match option {
            "a" => Self::Accessed,
            "B" => Self::Birthed,
            "c" => Self::Changed,
            _ => Self::Modified,
        }
    }

    fn get_file_time(self, metadata: &Metadata) -> std::io::Result<SystemTime> {
        match self {
            Self::Accessed => metadata.accessed(),
            Self::Birthed => metadata.created(),
            Self::Changed => metadata.changed(),
            Self::Modified => metadata.modified(),
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
    pub fn new(x_option: &str, y_option: &str, path_to_file: &str) -> Result<Self, Box<dyn Error>> {
        let metadata = fs::metadata(path_to_file)?;
        let x_option = NewerOptionType::from_str(x_option);
        let y_option = NewerOptionType::from_str(y_option);
        Ok(Self {
            x_option,
            y_option,
            given_modification_time: metadata.modified()?,
        })
    }

    fn matches_impl(&self, file_info: &WalkEntry) -> Result<bool, Box<dyn Error>> {
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
    fn matches(&self, file_info: &WalkEntry, _: &mut MatcherIO) -> bool {
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

    fn matches_impl(&self, file_info: &WalkEntry) -> Result<bool, Box<dyn Error>> {
        let this_time = self.newer_time_type.get_file_time(file_info.metadata()?)?;
        // `self.time` is in milliseconds and negative before 1970
        let offset = Duration::from_millis(self.time.unsigned_abs());
        let reference = if self.time >= 0 {
            UNIX_EPOCH.checked_add(offset)
        } else {
            UNIX_EPOCH.checked_sub(offset)
        };
        // Strictly newer, like GNU find. A reference outside what the platform
        // can store (e.g. before 1601 on Windows) is older or newer than any file.
        Ok(reference.map_or(self.time < 0, |reference| this_time > reference))
    }
}

impl Matcher for NewerTimeMatcher {
    fn matches(&self, file_info: &WalkEntry, _: &mut MatcherIO) -> bool {
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

/// Provide access to the *change* timestamp, since std::fs::Metadata doesn't expose it.
pub trait ChangeTime {
    /// Returns the time of the last change to the metadata.
    fn changed(&self) -> std::io::Result<SystemTime>;
}

#[cfg(unix)]
impl ChangeTime for Metadata {
    fn changed(&self) -> std::io::Result<SystemTime> {
        let ctime_sec = self.ctime();
        let ctime_nsec = self.ctime_nsec() as u32;
        let ctime = if ctime_sec >= 0 {
            UNIX_EPOCH + std::time::Duration::new(ctime_sec as u64, ctime_nsec)
        } else {
            UNIX_EPOCH - std::time::Duration::new(-ctime_sec as u64, ctime_nsec)
        };
        Ok(ctime)
    }
}

#[cfg(not(unix))]
impl ChangeTime for Metadata {
    fn changed(&self) -> std::io::Result<SystemTime> {
        // Rust's stdlib doesn't (yet) expose ChangeTime on Windows
        // https://github.com/rust-lang/rust/issues/121478
        Err(std::io::Error::from(std::io::ErrorKind::Unsupported))
    }
}

#[derive(Clone, Copy, Debug)]
pub enum FileTimeType {
    Accessed,
    Changed,
    Modified,
}

impl FileTimeType {
    fn get_file_time(self, metadata: &Metadata) -> std::io::Result<SystemTime> {
        match self {
            Self::Accessed => metadata.accessed(),
            Self::Changed => metadata.changed(),
            Self::Modified => metadata.modified(),
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
    fn matches(&self, file_info: &WalkEntry, matcher_io: &mut MatcherIO) -> bool {
        let start_time = get_time(matcher_io, self.today_start);
        match self.matches_impl(file_info, start_time) {
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
    fn matches_impl(
        &self,
        file_info: &WalkEntry,
        start_time: SystemTime,
    ) -> Result<bool, Box<dyn Error>> {
        let this_time = self.file_time_type.get_file_time(file_info.metadata()?)?;
        let mut is_negative = false;
        // durations can't be negative. So duration_since returns a duration
        // wrapped in an error if now < this_time.
        let age = match start_time.duration_since(this_time) {
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
        // If today_start is true, we should count it as 0 days old.
        // because today is 00:00:00, so we need to subtract 1 day.
        let negative_offset = if is_negative && !self.today_start {
            -1
        } else {
            0
        };

        let age_in_days = age_in_seconds / SECONDS_PER_DAY + negative_offset;
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
    fn matches(&self, file_info: &WalkEntry, matcher_io: &mut MatcherIO) -> bool {
        let start_time = get_time(matcher_io, self.today_start);
        match self.matches_impl(file_info, start_time) {
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
    fn matches_impl(
        &self,
        file_info: &WalkEntry,
        start_time: SystemTime,
    ) -> Result<bool, Box<dyn Error>> {
        let this_time = self.file_time_type.get_file_time(file_info.metadata()?)?;
        let mut is_negative = false;
        let age = match start_time.duration_since(this_time) {
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

/// `-used N`: the file was last accessed N days after its status last changed,
/// counting a part day as a whole one. Never true when the access is not later.
pub struct UsedMatcher {
    days: ComparableValue,
}

impl Matcher for UsedMatcher {
    fn matches(&self, file_info: &WalkEntry, _: &mut MatcherIO) -> bool {
        match self.matches_impl(file_info) {
            Err(e) => {
                writeln!(
                    &mut stderr(),
                    "Error getting access and change times for {}: {}",
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

impl UsedMatcher {
    fn matches_impl(&self, file_info: &WalkEntry) -> Result<bool, Box<dyn Error>> {
        let metadata = file_info.metadata()?;
        let accessed = FileTimeType::Accessed.get_file_time(metadata)?;
        let changed = FileTimeType::Changed.get_file_time(metadata)?;
        let Ok(used) = accessed.duration_since(changed) else {
            return Ok(false);
        };
        if used.is_zero() {
            return Ok(false);
        }
        let day = Duration::from_secs(SECONDS_PER_DAY as u64).as_nanos();
        let days = used.as_nanos().div_ceil(day) as u64;
        Ok(match self.days {
            ComparableValue::MoreThan(n) => days > n,
            ComparableValue::EqualTo(n) => days == n,
            ComparableValue::LessThan(n) => days <= n,
        })
    }

    pub fn new(days: ComparableValue) -> Self {
        Self { days }
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveTime;
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

        let matcher_for_new = NewerMatcher::new(
            &temp_dir.path().join(new_file_name).to_string_lossy(),
            Follow::Never,
        )
        .unwrap();
        let matcher_for_old =
            NewerMatcher::new(&old_file.path().to_string_lossy(), Follow::Never).unwrap();
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
    fn file_time_matcher_with_daystart() {
        // this file should already exist
        let file = get_dir_entry_for("test_data", "simple");

        let mut deps = FakeDependencies::new();
        let files_mtime = file.metadata().unwrap().modified().unwrap();

        let exactly_one_day_matcher =
            FileTimeMatcher::new(FileTimeType::Modified, ComparableValue::EqualTo(1), true);
        let more_than_one_day_matcher =
            FileTimeMatcher::new(FileTimeType::Modified, ComparableValue::MoreThan(1), true);
        let less_than_one_day_matcher =
            FileTimeMatcher::new(FileTimeType::Modified, ComparableValue::LessThan(1), true);
        let zero_day_matcher =
            FileTimeMatcher::new(FileTimeType::Modified, ComparableValue::EqualTo(0), true);

        // set "now" to 3 days after the file was modified.
        // Because daystart affects the time when the calculation starts,
        // in order to avoid complicated assertions, it is set to 3 days later.
        deps.set_time(files_mtime + Duration::new(3 * SECONDS_PER_DAY as u64, 0));
        assert!(
            !exactly_one_day_matcher.matches(&file, &mut deps.new_matcher_io()),
            "3 day old file shouldn't match exactly 1 day old"
        );
        assert!(
            more_than_one_day_matcher.matches(&file, &mut deps.new_matcher_io()),
            "3 day old file should match more than 1 day old"
        );
        assert!(
            !less_than_one_day_matcher.matches(&file, &mut deps.new_matcher_io()),
            "3 day old file shouldn't match less than 1 day old"
        );
        assert!(
            !zero_day_matcher.matches(&file, &mut deps.new_matcher_io()),
            "3 day old file shouldn't match exactly 0 days old"
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
            zero_day_matcher.matches(&file, &mut deps.new_matcher_io()),
            "future-modified file should match exactly 0 days old"
        );
    }

    #[test]
    fn get_local_midnight() {
        let deps = FakeDependencies::new();
        let midnight = get_time(&mut deps.new_matcher_io(), true);

        let midnight = DateTime::<Local>::from(midnight);
        assert_eq!(midnight.time(), NaiveTime::from_hms_opt(0, 0, 0).unwrap());
    }

    #[test]
    fn file_time_matcher_modified_changed_accessed() {
        let temp_dir = Builder::new()
            .prefix("file_time_matcher_modified_changed_accessed")
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

        if let Ok(creation_time) = metadata.changed() {
            test_matcher_for_file_time_type(&file_info, creation_time, FileTimeType::Changed);
        }

        if let Ok(modified_time) = metadata.modified() {
            test_matcher_for_file_time_type(&file_info, modified_time, FileTimeType::Modified);
        }
    }

    /// helper function for `file_time_matcher_modified_changed_accessed`
    fn test_matcher_for_file_time_type(
        file_info: &WalkEntry,
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
        let options = [
            "a",
            #[cfg(not(target_os = "linux"))]
            "B",
            #[cfg(unix)]
            "c",
            "m",
        ];

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
                let matcher =
                    NewerOptionMatcher::new(x_option, y_option, &old_file.path().to_string_lossy());

                assert!(
                    matcher
                        .unwrap()
                        .matches(&new_file, &mut deps.new_matcher_io()),
                    "new_file should be newer than old_dir"
                );
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

        #[cfg(unix)]
        {
            let inode_changed_matcher = NewerTimeMatcher::new(NewerOptionType::Changed, time);
            // Steps to change inode:
            // 1. Copy and rename the file
            // 2. Delete the old file
            // 3. Change the new file name to the old file name
            let _ =
                File::create(temp_dir.path().join("inode_test_file")).expect("create temp file");
            let _ = fs::copy("inode_test_file", "new_inode_test_file");
            let _ = fs::remove_file("inode_test_file");
            let _ = fs::rename("new_inode_test_file", "inode_test_file");
            let file_info =
                get_dir_entry_for(&temp_dir.path().to_string_lossy(), "inode_test_file");
            assert!(
                inode_changed_matcher.matches(&file_info, &mut deps.new_matcher_io()),
                "file inode changed time should after 'std_time'"
            );
        }
    }

    #[test]
    fn newer_time_matcher_is_strict_and_signed() {
        let temp_dir = Builder::new()
            .prefix("newer_time_matcher_is_strict_and_signed")
            .tempdir()
            .unwrap();
        let path = temp_dir.path().join("foo");
        let file = File::create(&path).expect("create temp file");
        let deps = FakeDependencies::new();
        let newer = |file_time: SystemTime, millis: i64| {
            file.set_modified(file_time).unwrap();
            let file_info = get_dir_entry_for(&temp_dir.path().to_string_lossy(), "foo");
            NewerTimeMatcher::new(NewerOptionType::Modified, millis)
                .matches(&file_info, &mut deps.new_matcher_io())
        };

        // A file with the reference time itself is not newer.
        let t = UNIX_EPOCH + Duration::from_secs(1_000_000_000);
        assert!(!newer(t, 1_000_000_000_000));
        assert!(newer(t, 999_999_999_999));

        // Times before 1970 keep their sign.
        let t = UNIX_EPOCH - Duration::from_secs(300_000_000);
        assert!(!newer(t, -200_000_000_000));
        assert!(newer(t, -400_000_000_000));

        // References far outside any file time don't panic.
        assert!(newer(t, i64::MIN));
        assert!(!newer(t, i64::MAX));
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
        for time_type in &[
            FileTimeType::Accessed,
            FileTimeType::Changed,
            FileTimeType::Modified,
        ] {
            let more_matcher =
                FileAgeRangeMatcher::new(*time_type, ComparableValue::MoreThan(1), true);
            assert!(
                !more_matcher.matches(&new_file, &mut FakeDependencies::new().new_matcher_io()),
                "{}",
                format!(
                    "more minutes old file should not match more than 1 minute old in {} test.",
                    match *time_type {
                        FileTimeType::Accessed => "accessed",
                        FileTimeType::Changed => "changed",
                        FileTimeType::Modified => "modified",
                    }
                )
            );
        }

        // less test
        // mocks:
        // - find test_data/simple -amin -1
        // - find test_data/simple -cmin -1
        // - find test_data/simple -mmin -1
        // Means to find files accessed / modified less than 1 minute ago.
        for time_type in &[
            FileTimeType::Accessed,
            #[cfg(unix)]
            FileTimeType::Changed,
            FileTimeType::Modified,
        ] {
            let less_matcher =
                FileAgeRangeMatcher::new(*time_type, ComparableValue::LessThan(1), true);
            assert!(
                less_matcher.matches(&new_file, &mut FakeDependencies::new().new_matcher_io()),
                "{}",
                format!(
                    "less minutes old file should match less than 1 minute old in {} test.",
                    match *time_type {
                        FileTimeType::Accessed => "accessed",
                        FileTimeType::Changed => "changed",
                        FileTimeType::Modified => "modified",
                    }
                )
            );
        }

        // catch file error
        let _ = fs::remove_file(&*new_file.path().to_string_lossy());
        let matcher =
            FileAgeRangeMatcher::new(FileTimeType::Modified, ComparableValue::MoreThan(1), true);
        assert!(
            !matcher.matches(&new_file, &mut FakeDependencies::new().new_matcher_io()),
            "The correct situation is that the file reading here cannot be successful."
        );
    }

    #[test]
    // The change time is only read on Unix.
    #[cfg(unix)]
    fn used_matcher() {
        let temp_dir = Builder::new().prefix("used_matcher").tempdir().unwrap();
        let later = temp_dir.path().join("later");
        let earlier = temp_dir.path().join("earlier");
        File::create(&later).unwrap();
        File::create(&earlier).unwrap();
        // Setting the access time also sets the change time to now.
        let now = SystemTime::now();
        let hour = Duration::from_secs(3600);
        filetime::set_file_atime(&later, (now + 25 * hour).into()).unwrap();
        filetime::set_file_atime(&earlier, (now - 25 * hour).into()).unwrap();
        let later = get_dir_entry_for(temp_dir.path().to_str().unwrap(), "later");
        let earlier = get_dir_entry_for(temp_dir.path().to_str().unwrap(), "earlier");
        let deps = FakeDependencies::new();

        // 25 hours counts as 2 days
        for (days, expected) in [
            (ComparableValue::EqualTo(2), true),
            (ComparableValue::EqualTo(1), false),
            (ComparableValue::MoreThan(1), true),
            (ComparableValue::MoreThan(2), false),
            (ComparableValue::LessThan(2), true),
            (ComparableValue::LessThan(1), false),
        ] {
            let label = format!("{days:?}");
            let matcher = UsedMatcher::new(days);
            assert_eq!(
                matcher.matches(&later, &mut deps.new_matcher_io()),
                expected,
                "{label}"
            );
            assert!(
                !matcher.matches(&earlier, &mut deps.new_matcher_io()),
                "{label}"
            );
        }
    }
}
