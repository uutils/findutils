// This file is part of the uutils findutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use super::Matcher;

#[cfg(unix)]
use nix::unistd::User;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

pub struct UserMatcher {
    uid: Option<u32>,
}

impl UserMatcher {
    #[cfg(unix)]
    pub fn new(user: String) -> UserMatcher {
        // get uid from user name
        let Ok(user) = User::from_name(user.as_str()) else {
            return UserMatcher { uid: None };
        };

        let Some(user) = user else {
            // This if branch is to determine whether a certain user exists in the system.
            // If a certain user does not exist in the system,
            // the result will need to be returned according to
            // the flag bit of whether to invert the result.
            return UserMatcher { uid: None };
        };

        UserMatcher {
            uid: Some(user.uid.as_raw()),
        }
    }

    #[cfg(windows)]
    pub fn new(_user: String) -> UserMatcher {
        UserMatcher { uid: None }
    }

    pub fn uid(&self) -> &Option<u32> {
        &self.uid
    }
}

impl Matcher for UserMatcher {
    #[cfg(unix)]
    fn matches(&self, file_info: &walkdir::DirEntry, _: &mut super::MatcherIO) -> bool {
        let Ok(metadata) = file_info.path().metadata() else {
            return false;
        };

        let file_uid = metadata.uid();

        // When matching the -user parameter in find/matcher/mod.rs,
        // it has been judged that the user does not exist and an error is returned.
        // So use unwarp() directly here.
        self.uid.unwrap() == file_uid
    }

    #[cfg(windows)]
    fn matches(&self, _file_info: &walkdir::DirEntry, _: &mut super::MatcherIO) -> bool {
        false
    }
}

pub struct NoUserMatcher {}

impl Matcher for NoUserMatcher {
    #[cfg(unix)]
    fn matches(&self, file_info: &walkdir::DirEntry, _: &mut super::MatcherIO) -> bool {
        use nix::unistd::Uid;

        if file_info.path().is_symlink() {
            return false;
        }

        let Ok(metadata) = file_info.path().metadata() else {
            return true;
        };

        let Ok(uid) = User::from_uid(Uid::from_raw(metadata.uid())) else {
            return true;
        };

        let Some(_user) = uid else {
            return true;
        };

        false
    }

    #[cfg(windows)]
    fn matches(&self, _file_info: &walkdir::DirEntry, _: &mut super::MatcherIO) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(unix)]
    fn test_user_matcher() {
        use crate::find::matchers::{tests::get_dir_entry_for, user::UserMatcher, Matcher};
        use crate::find::tests::FakeDependencies;
        use chrono::Local;
        use nix::unistd::{Uid, User};
        use std::fs::File;
        use std::os::unix::fs::MetadataExt;
        use tempfile::Builder;

        let deps = FakeDependencies::new();
        let mut matcher_io = deps.new_matcher_io();

        let temp_dir = Builder::new().prefix("user_matcher").tempdir().unwrap();
        let foo_path = temp_dir.path().join("foo");
        let _ = File::create(foo_path).expect("create temp file");
        let file_info = get_dir_entry_for(&temp_dir.path().to_string_lossy(), "foo");
        let file_uid = file_info.path().metadata().unwrap().uid();
        let file_user = User::from_uid(Uid::from_raw(file_uid))
            .unwrap()
            .unwrap()
            .name;

        let matcher = UserMatcher::new(file_user.clone());
        assert!(
            matcher.matches(&file_info, &mut matcher_io),
            "user should be the same"
        );

        // Testing a non-existent group name
        let time_string = Local::now().format("%Y%m%d%H%M%S").to_string();
        let matcher = UserMatcher::new(time_string.clone());
        assert!(
            matcher.uid().is_none(),
            "user {} should not be the same",
            time_string
        );
    }
}
