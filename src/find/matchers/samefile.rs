// This file is part of the uutils findutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::path::PathBuf;

use super::Matcher;

pub struct SameFileMatcher {
    path: PathBuf,
}

impl SameFileMatcher {
    pub fn new(path: PathBuf) -> SameFileMatcher {
        SameFileMatcher { path }
    }
}

impl Matcher for SameFileMatcher {
    #[cfg(unix)]
    fn matches(&self, file_info: &walkdir::DirEntry, _matcher_io: &mut super::MatcherIO) -> bool {
        uucore::fs::paths_refer_to_same_file(file_info.path(), &self.path, true)
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
        let hard_link_file = get_dir_entry_for("test_data/links", "hard_link");
        let matcher = SameFileMatcher::new(file.into_path());

        let deps = FakeDependencies::new();
        assert!(matcher.matches(&hard_link_file, &mut deps.new_matcher_io()));
    }
}
