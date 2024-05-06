use super::Matcher;

#[cfg(unix)]
use nix::unistd::Group;
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
        let file_group = file_info.path().metadata().unwrap().gid();

        // get gid from group name
        let Ok(group) = Group::from_name(self.group.as_str()) else {
            return false;
        };

        let Some(group) = group else {
            return false;
        };

        let gid = group.gid.as_raw();
        if self.reverse {
            file_group != gid
        } else {
            file_group == gid
        }
    }

    #[cfg(windows)]
    fn matches(&self, file_info: &walkdir::DirEntry, _: &mut super::MatcherIO) -> bool {
        // The user group acquisition function for Windows systems is not implemented in MetadataExt,
        // so it is somewhat difficult to implement it. :(
        true
    }
}
