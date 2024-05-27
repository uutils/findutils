// This file is part of the uutils findutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::fs::Metadata;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

use super::Matcher;

pub struct SameFileMatcher {
    metadata: Metadata,
}

impl SameFileMatcher {
    pub fn new(metadata: Metadata) -> SameFileMatcher {
        SameFileMatcher { metadata }
    }
}

impl Matcher for SameFileMatcher {
    #[cfg(unix)]
    fn matches(&self, file_info: &walkdir::DirEntry, _matcher_io: &mut super::MatcherIO) -> bool {
        let meta = file_info.metadata().unwrap();

        if meta.dev() != self.metadata.dev() {
            return false;
        }

        meta.ino() == self.metadata.ino()
    }

    #[cfg(not(unix))]
    fn matches(&self, _file_info: &walkdir::DirEntry, _matcher_io: &mut super::MatcherIO) -> bool {
        // FIXME
        // MetadataExt under Windows system does not have an interface for obtaining inode,
        // so the implementation of this function needs to introduce other libraries or use unsafe code.
        false
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    #[test]
    #[cfg(unix)]
    fn test_samefile() {
        use crate::find::{
            matchers::{samefile::SameFileMatcher, tests::get_dir_entry_for, Matcher},
            tests::FakeDependencies,
        };

        // remove file if hard link file exist.
        // But you can't delete a file that doesn't exist,
        // so ignore the error returned here.
        let _ = fs::remove_file("test_data/links/hard_link");
        fs::hard_link("test_data/links/abbbc", "test_data/links/hard_link").unwrap();

        let file = get_dir_entry_for("test_data/links", "abbbc");
        let file_metadata = file
            .metadata()
            .expect("Failed to get original file metadata");
        let hard_link_file = get_dir_entry_for("test_data/links", "hard_link");
        let matcher = SameFileMatcher::new(file_metadata);

        let deps = FakeDependencies::new();
        assert!(matcher.matches(&hard_link_file, &mut deps.new_matcher_io()));
    }
}
