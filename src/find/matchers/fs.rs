// This file is part of the uutils findutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use super::Matcher;

pub struct FileSystemMatcher {
    fs_text: String,
}

impl FileSystemMatcher {
    pub fn new(fs_text: String) -> Self {
        Self { fs_text }
    }
}

impl Matcher for FileSystemMatcher {
    fn matches(&self, file_info: &walkdir::DirEntry, _: &mut super::MatcherIO) -> bool {
        #[cfg(not(unix))]
        {
            false
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let dev_id = file_info
                .metadata()
                .expect("Could not get metadata")
                .dev()
                .to_string();
            let fs_list =
                uucore::fsext::read_fs_list().expect("Could not find the filesystem info");

            fs_list
                .into_iter()
                .find(|fs| fs.dev_id == dev_id)
                .map_or_else(String::new, |fs| fs.fs_type)
                .contains(&self.fs_text)
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(unix)]
    fn test_fs_matcher() {
        use crate::find::{
            matchers::{tests::get_dir_entry_for, Matcher},
            tests::FakeDependencies,
        };
        use std::{fs::File, os::unix::fs::MetadataExt};
        use tempfile::Builder;

        let deps = FakeDependencies::new();
        let mut matcher_io = deps.new_matcher_io();

        let temp_dir = Builder::new().prefix("fs_matcher").tempdir().unwrap();
        let foo_path = temp_dir.path().join("foo");
        let _ = File::create(foo_path).expect("create temp file");
        let file_info = get_dir_entry_for(&temp_dir.path().to_string_lossy(), "foo");

        let dev_id = file_info
            .metadata()
            .expect("Could not get metadata")
            .dev()
            .to_string();
        let fs_list = uucore::fsext::read_fs_list().expect("Could not find the filesystem info");
        let target_fs_type = fs_list
            .into_iter()
            .find(|fs| fs.dev_id == dev_id)
            .map_or_else(String::new, |fs| fs.fs_type);

        // should match fs type
        let matcher = super::FileSystemMatcher::new(target_fs_type.clone());
        assert!(
            matcher.matches(&file_info, &mut matcher_io),
            "{} should match {}",
            file_info.path().to_string_lossy(),
            target_fs_type
        );

        // should not match fs type
        let matcher = super::FileSystemMatcher::new(target_fs_type.clone() + "foo");
        assert!(
            !matcher.matches(&file_info, &mut matcher_io),
            "{} should not match {}",
            file_info.path().to_string_lossy(),
            target_fs_type
        );
    }
}
