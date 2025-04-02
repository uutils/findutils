// This file is part of the uutils findutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use super::{Matcher, MatcherIO, WalkEntry};

#[cfg(unix)]
use nix::unistd::Group;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

pub struct GroupMatcher {
    gid: Option<u32>,
}

impl GroupMatcher {
    #[cfg(unix)]
    pub fn from_group_name(group: &str) -> Self {
        // get gid from group name
        let Ok(group) = Group::from_name(group) else {
            return Self { gid: None };
        };

        let Some(group) = group else {
            // This if branch is to determine whether a certain group exists in the system.
            // If a certain group does not exist in the system,
            // the result will need to be returned according to
            // the flag bit of whether to invert the result.
            return Self { gid: None };
        };

        Self {
            gid: Some(group.gid.as_raw()),
        }
    }

    #[cfg(unix)]
    pub fn from_gid(gid: u32) -> Self {
        Self { gid: Some(gid) }
    }

    #[cfg(windows)]
    pub fn from_group_name(_group: &str) -> GroupMatcher {
        GroupMatcher { gid: None }
    }

    #[cfg(windows)]
    pub fn from_gid(_gid: u32) -> GroupMatcher {
        GroupMatcher { gid: None }
    }

    pub fn gid(&self) -> &Option<u32> {
        &self.gid
    }
}

impl Matcher for GroupMatcher {
    #[cfg(unix)]
    fn matches(&self, file_info: &WalkEntry, _: &mut MatcherIO) -> bool {
        let Ok(metadata) = file_info.metadata() else {
            return false;
        };

        let file_gid = metadata.gid();

        // When matching the -group parameter in find/matcher/mod.rs,
        // it has been judged that the group does not exist and an error is returned.
        // So use unwarp() directly here.
        self.gid.unwrap() == file_gid
    }

    #[cfg(windows)]
    fn matches(&self, _file_info: &WalkEntry, _: &mut MatcherIO) -> bool {
        // The user group acquisition function for Windows systems is not implemented in MetadataExt,
        // so it is somewhat difficult to implement it. :(
        false
    }
}

pub struct NoGroupMatcher {}

impl Matcher for NoGroupMatcher {
    #[cfg(unix)]
    fn matches(&self, file_info: &WalkEntry, _: &mut MatcherIO) -> bool {
        use nix::unistd::Gid;

        if file_info.path().is_symlink() {
            return false;
        }

        let Ok(metadata) = file_info.metadata() else {
            return true;
        };

        let Ok(gid) = Group::from_gid(Gid::from_raw(metadata.gid())) else {
            return true;
        };

        let Some(_group) = gid else {
            return true;
        };

        false
    }

    #[cfg(windows)]
    fn matches(&self, _file_info: &WalkEntry, _: &mut MatcherIO) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(unix)]
    fn test_group_matcher() {
        use crate::find::matchers::{group::GroupMatcher, tests::get_dir_entry_for, Matcher};
        use crate::find::tests::FakeDependencies;
        use chrono::Local;
        use nix::unistd::{Gid, Group};
        use std::fs::File;
        use std::os::unix::fs::MetadataExt;
        use tempfile::Builder;

        let deps = FakeDependencies::new();
        let mut matcher_io = deps.new_matcher_io();

        let temp_dir = Builder::new().prefix("group_matcher").tempdir().unwrap();
        let foo_path = temp_dir.path().join("foo");
        let _ = File::create(foo_path).expect("create temp file");
        let file_info = get_dir_entry_for(&temp_dir.path().to_string_lossy(), "foo");
        let file_gid = file_info.metadata().unwrap().gid();
        let file_group = Group::from_gid(Gid::from_raw(file_gid))
            .unwrap()
            .unwrap()
            .name;

        let matcher = super::GroupMatcher::from_group_name(file_group.as_str());
        assert!(
            matcher.matches(&file_info, &mut matcher_io),
            "group should match"
        );

        // Testing a non-existent group name
        let time_string = Local::now().format("%Y%m%d%H%M%S").to_string();
        let matcher = GroupMatcher::from_group_name(time_string.as_str());
        assert!(
            matcher.gid().is_none(),
            "group name {} should not exist",
            time_string
        );

        // Testing group id
        let matcher = GroupMatcher::from_gid(file_gid);
        assert!(
            matcher.gid().is_some(),
            "group id {} should exist",
            file_gid
        );
        assert!(
            matcher.matches(&file_info, &mut matcher_io),
            "group id should match"
        );
    }
}
