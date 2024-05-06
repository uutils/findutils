use super::Matcher;

#[cfg(unix)]
use nix::unistd::User;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;

pub struct UserMatcher {
    reverse: bool,
    user: String,
}

impl UserMatcher {
    pub fn new(user: String, reverse: bool) -> UserMatcher {
        UserMatcher { reverse, user }
    }
}

impl Matcher for UserMatcher {
    #[cfg(unix)]
    fn matches(&self, file_info: &walkdir::DirEntry, _: &mut super::MatcherIO) -> bool {
        let file_uid = file_info.path().metadata().unwrap().uid();

        // get uid from user name
        let Ok(user) = User::from_name(self.user.as_str()) else {
            return false;
        };

        let Some(user) = user else {
            return false;
        };

        let uid = user.uid.as_raw();
        if self.reverse {
            file_uid != uid
        } else {
            file_uid == uid
        }
    }

    #[cfg(windows)]
    fn matches(&self, _file_info: &walkdir::DirEntry, _: &mut super::MatcherIO) -> bool {
        // The user group acquisition function for Windows systems is not implemented in MetadataExt,
        // so it is somewhat difficult to implement it. :(
        true
    }
}
