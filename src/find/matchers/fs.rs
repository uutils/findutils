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
        let statfs = match nix::sys::statfs::statfs(file_info.path()) {
            Ok(statfs) => statfs,
            Err(_) => return false,
        };

        // filesystem type id to name
        let magic_number = statfs.filesystem_type();
        let fs_type = uucore::fsext::pretty_fstype(magic_number.0);

        fs_type == self.fs_text
    }
}
