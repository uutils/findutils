// This file is part of the uutils findutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use super::Matcher;
use std::error::Error;
use std::path::Path;
use uucore::fs::FileInformation;

pub struct SameFileMatcher {
    info: FileInformation,
}

impl SameFileMatcher {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
        let info = FileInformation::from_path(path, false)?;
        Ok(Self { info })
    }
}

impl Matcher for SameFileMatcher {
    fn matches(&self, file_info: &walkdir::DirEntry, _matcher_io: &mut super::MatcherIO) -> bool {
        if let Ok(info) = FileInformation::from_path(file_info.path(), false) {
            info == self.info
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    #[test]
    fn test_samefile() {
        use crate::find::{
            matchers::{samefile::SameFileMatcher, tests::get_dir_entry_for, Matcher},
            tests::FakeDependencies,
        };

        // remove file if hard link file exist.
        // But you can't delete a file that doesn't exist,
        // so ignore the error returned here.
        let _ = fs::remove_file("test_data/links/hard_link");

        assert!(SameFileMatcher::new("test_data/links/hard_link").is_err());

        fs::hard_link("test_data/links/abbbc", "test_data/links/hard_link").unwrap();

        let file = get_dir_entry_for("test_data/links", "abbbc");
        let hard_link_file = get_dir_entry_for("test_data/links", "hard_link");
        let matcher = SameFileMatcher::new(file.into_path()).unwrap();

        let deps = FakeDependencies::new();
        assert!(matcher.matches(&hard_link_file, &mut deps.new_matcher_io()));
    }
}
