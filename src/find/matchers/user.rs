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
        let Ok(metadata) = file_info.path().metadata() else {
            return false;
        };

        let file_uid = metadata.uid();

        // get uid from user name
        let Ok(user) = User::from_name(self.user.as_str()) else {
            return false;
        };

        let Some(user) = user else {
            // This if branch is to determine whether a certain user exists in the system.
            // If a certain user does not exist in the system,
            // the result will need to be returned according to
            // the flag bit of whether to invert the result.
            return self.reverse;
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

#[cfg(test)]
mod tests {
    use crate::find::tests::FakeDependencies;

    #[test]
    #[cfg(unix)]
    fn test_user_matcher() {
        use std::fs::File;

        use crate::find::matchers::{tests::get_dir_entry_for, user::UserMatcher, Matcher};
        use nix::unistd::{Uid, User};
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

        let matcher = UserMatcher::new(file_user.clone(), false);
        assert!(
            matcher.matches(&file_info, &mut matcher_io),
            "user should be the same"
        );

        let matcher_reverse = UserMatcher::new(file_user.clone(), true);
        assert!(
            !matcher_reverse.matches(&file_info, &mut matcher_io),
            "user should not be the same"
        );
    }
}
