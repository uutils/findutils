// This file is part of the uutils findutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::path::Path;
use std::{
    error::Error,
    io::{stderr, Write},
};

use super::Matcher;

/// Get the filesystem type of a file.
/// 1. get the metadata of the file
/// 2. get the device ID of the metadata
/// 3. search the filesystem list
///
/// Returns an empty string when no file system list matches.
///
/// # Errors
/// Returns an error if the metadata could not be read.
/// Returns an error if the filesystem list could not be read.
///
/// This is only supported on Unix.
#[cfg(unix)]
pub fn get_file_system_type(path: &Path) -> Result<String, Box<dyn Error>> {
    use std::os::unix::fs::MetadataExt;

    let metadata = match path.metadata() {
        Ok(metadata) => metadata,
        Err(err) => Err(err)?,
    };
    let dev_id = metadata.dev().to_string();
    let fs_list = match uucore::fsext::read_fs_list() {
        Ok(fs_list) => fs_list,
        Err(err) => Err(err)?,
    };
    let result = fs_list
        .into_iter()
        .find(|fs| fs.dev_id == dev_id)
        .map_or_else(String::new, |fs| fs.fs_type);

    Ok(result)
}

/// This matcher handles the -fstype argument.
/// It matches the filesystem type of the file.
///
/// This is only supported on Unix.
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
            match get_file_system_type(file_info.path()) {
                Ok(result) => result == self.fs_text,
                Err(_) => {
                    writeln!(
                        &mut stderr(),
                        "Error getting filesystem type for {}",
                        file_info.path().to_string_lossy()
                    )
                    .unwrap();

                    false
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(unix)]
    fn test_fs_matcher() {
        use crate::find::{
            matchers::{fs::get_file_system_type, tests::get_dir_entry_for, Matcher},
            tests::FakeDependencies,
        };
        use std::fs::File;
        use tempfile::Builder;

        let deps = FakeDependencies::new();
        let mut matcher_io = deps.new_matcher_io();

        // create temp file and get its fs type
        // We pass this file and the corresponding file system type into the Matcher for comparison.
        let temp_dir = Builder::new().prefix("fs_matcher").tempdir().unwrap();
        let foo_path = temp_dir.path().join("foo");
        let _ = File::create(foo_path).expect("create temp file");
        let file_info = get_dir_entry_for(&temp_dir.path().to_string_lossy(), "foo");

        let target_fs_type = get_file_system_type(file_info.path()).unwrap();

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
