use super::Matcher;

#[cfg(unix)]
use nix::unistd::Group;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;

pub struct GroupMatcher {
    reverse: bool,
    group: String,
}

impl GroupMatcher {
    pub fn new(group: String, reverse: bool) -> GroupMatcher {
        GroupMatcher { reverse, group }
    }
}

impl Matcher for GroupMatcher {
    #[cfg(unix)]
    fn matches(&self, file_info: &walkdir::DirEntry, _: &mut super::MatcherIO) -> bool {
        let Ok(metadata) = file_info.path().metadata() else {
            return false;
        };

        let file_gid = metadata.gid();

        // get gid from group name
        let Ok(group) = Group::from_name(self.group.as_str()) else {
            return false;
        };

        let Some(group) = group else {
            // This if branch is to determine whether a certain group exists in the system.
            // If a certain group does not exist in the system,
            // the result will need to be returned according to
            // the flag bit of whether to invert the result.
            return self.reverse;
        };

        let gid = group.gid.as_raw();
        if self.reverse {
            file_gid != gid
        } else {
            file_gid == gid
        }
    }

    #[cfg(windows)]
    fn matches(&self, _file_info: &walkdir::DirEntry, _: &mut super::MatcherIO) -> bool {
        // The user group acquisition function for Windows systems is not implemented in MetadataExt,
        // so it is somewhat difficult to implement it. :(
        true
    }
}

#[cfg(test)]
mod tests {
    use crate::find::tests::FakeDependencies;

    #[test]
    #[cfg(unix)]
    fn test_group_matcher() {
        use std::fs::File;

        use crate::find::matchers::{tests::get_dir_entry_for, Matcher};
        use nix::unistd::{Gid, Group};
        use std::os::unix::fs::MetadataExt;
        use tempfile::Builder;

        let deps = FakeDependencies::new();
        let mut matcher_io = deps.new_matcher_io();

        let temp_dir = Builder::new().prefix("group_matcher").tempdir().unwrap();
        let foo_path = temp_dir.path().join("foo");
        let _ = File::create(foo_path).expect("create temp file");
        let file_info = get_dir_entry_for(&temp_dir.path().to_string_lossy(), "foo");
        let file_gid = file_info.path().metadata().unwrap().gid();
        let file_group = Group::from_gid(Gid::from_raw(file_gid))
            .unwrap()
            .unwrap()
            .name;

        let matcher = super::GroupMatcher::new(file_group.clone(), false);
        assert!(
            matcher.matches(&file_info, &mut matcher_io),
            "group should match"
        );

        let matcher_reverse = super::GroupMatcher::new(file_group.clone(), true);
        assert!(
            !matcher_reverse.matches(&file_info, &mut matcher_io),
            "group should not match in reverse predicate"
        );
    }
}
