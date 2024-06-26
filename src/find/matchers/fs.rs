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
