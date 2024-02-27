// Copyright 2022 Tavian Barnes
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

use std::error::Error;
use walkdir::DirEntry;

use super::{ComparableValue, Matcher, MatcherIO};

/// Inode number matcher.
pub struct InodeMatcher {
    ino: ComparableValue,
}

impl InodeMatcher {
    #[cfg(unix)]
    pub fn new(ino: ComparableValue) -> Result<Self, Box<dyn Error>> {
        Ok(Self { ino })
    }

    #[cfg(not(unix))]
    pub fn new(_ino: ComparableValue) -> Result<Self, Box<dyn Error>> {
        Err(From::from(
            "Inode numbers are not available on this platform",
        ))
    }
}

impl Matcher for InodeMatcher {
    #[cfg(unix)]
    fn matches(&self, file_info: &DirEntry, _: &mut MatcherIO) -> bool {
        match file_info.metadata() {
            Ok(metadata) => self.ino.matches(metadata.ino()),
            Err(_) => false,
        }
    }

    #[cfg(not(unix))]
    fn matches(&self, _: &DirEntry, _: &mut MatcherIO) -> bool {
        unreachable!("Inode numbers are not available on this platform")
    }
}

/// Link count matcher.
pub struct LinksMatcher {
    nlink: ComparableValue,
}

impl LinksMatcher {
    #[cfg(unix)]
    pub fn new(nlink: ComparableValue) -> Result<Self, Box<dyn Error>> {
        Ok(Self { nlink })
    }

    #[cfg(not(unix))]
    pub fn new(_nlink: ComparableValue) -> Result<Self, Box<dyn Error>> {
        Err(From::from("Link counts are not available on this platform"))
    }
}

impl Matcher for LinksMatcher {
    #[cfg(unix)]
    fn matches(&self, file_info: &DirEntry, _: &mut MatcherIO) -> bool {
        match file_info.metadata() {
            Ok(metadata) => self.nlink.matches(metadata.nlink()),
            Err(_) => false,
        }
    }

    #[cfg(not(unix))]
    fn matches(&self, _: &DirEntry, _: &mut MatcherIO) -> bool {
        unreachable!("Link counts are not available on this platform")
    }
}

#[cfg(test)]
#[cfg(unix)]
mod tests {
    use super::*;

    use crate::find::matchers::tests::get_dir_entry_for;
    use crate::find::tests::FakeDependencies;

    #[test]
    fn inode_matcher() {
        let file_info = get_dir_entry_for("test_data/simple", "abbbc");
        let metadata = file_info.metadata().unwrap();
        let deps = FakeDependencies::new();

        let matcher = InodeMatcher::new(ComparableValue::EqualTo(metadata.ino())).unwrap();
        assert!(
            matcher.matches(&file_info, &mut deps.new_matcher_io()),
            "inode number should match"
        );

        let matcher = InodeMatcher::new(ComparableValue::LessThan(metadata.ino())).unwrap();
        assert!(
            !matcher.matches(&file_info, &mut deps.new_matcher_io()),
            "inode number should not match"
        );

        let matcher = InodeMatcher::new(ComparableValue::MoreThan(metadata.ino())).unwrap();
        assert!(
            !matcher.matches(&file_info, &mut deps.new_matcher_io()),
            "inode number should not match"
        );
    }

    #[test]
    fn links_matcher() {
        let file_info = get_dir_entry_for("test_data/simple", "abbbc");
        let deps = FakeDependencies::new();

        let matcher = LinksMatcher::new(ComparableValue::EqualTo(1)).unwrap();
        assert!(
            matcher.matches(&file_info, &mut deps.new_matcher_io()),
            "link count should match"
        );

        let matcher = LinksMatcher::new(ComparableValue::LessThan(1)).unwrap();
        assert!(
            !matcher.matches(&file_info, &mut deps.new_matcher_io()),
            "link count should not match"
        );

        let matcher = LinksMatcher::new(ComparableValue::MoreThan(1)).unwrap();
        assert!(
            !matcher.matches(&file_info, &mut deps.new_matcher_io()),
            "link count should not match"
        );
    }
}
