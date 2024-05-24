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
        meta.ino() == self.metadata.ino()
    }

    #[cfg(not(unix))]
    fn matches(&self, _file_info: &walkdir::DirEntry, _matcher_io: &mut super::MatcherIO) -> bool {
        false
    }
}
