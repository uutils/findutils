/*
 * This file is part of the uutils findutils package.
 *
 * (c) Arcterus <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

use std::fs::{self, FileType};
use std::io::{self, stderr, Write};
use std::path::Path;

use walkdir::DirEntry;

use super::{Matcher, MatcherIO};

pub struct DeleteMatcher;

impl DeleteMatcher {
    pub fn new() -> DeleteMatcher {
        DeleteMatcher
    }

    pub fn new_box() -> io::Result<Box<dyn Matcher>> {
        Ok(Box::new(DeleteMatcher::new()))
    }

    fn delete(&self, file_path: &Path, file_type: FileType) -> io::Result<()> {
        if file_type.is_dir() {
            fs::remove_dir(file_path)
        } else {
            fs::remove_file(file_path)
        }
    }
}

impl Matcher for DeleteMatcher {
    fn matches(&self, file_info: &DirEntry, _: &mut MatcherIO) -> bool {
        let path = file_info.path();
        let path_str = path.to_string_lossy();

        // This is a quirk in find's traditional semantics probably due to
        // POSIX rmdir() not accepting "." (EINVAL). std::fs::remove_dir()
        // inherits the same behavior, so no reason to buck tradition.
        if path_str == "." {
            return false;
        }

        match self.delete(path, file_info.file_type()) {
            Ok(_) => true,
            Err(e) => {
                writeln!(&mut stderr(), "Failed to delete {}: {}", path_str, e).unwrap();
                false
            }
        }
    }

    fn has_side_effects(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use tempdir::TempDir;

    use super::*;
    use crate::find::matchers::tests::get_dir_entry_for;
    use crate::find::tests::FakeDependencies;

    #[test]
    fn delete_matcher() {
        let matcher = DeleteMatcher::new();
        let deps = FakeDependencies::new();

        let temp_dir = TempDir::new_in("test_data", "delete_matcher").expect("made temp dir");
        let temp_dir_path = temp_dir.path().to_string_lossy();
        File::create(temp_dir.path().join("test")).expect("created test file");
        let test_entry = get_dir_entry_for(&temp_dir_path, "test");
        assert!(
            matcher.matches(&test_entry, &mut deps.new_matcher_io()),
            "DeleteMatcher should match a simple file",
        );
        assert!(
            !temp_dir.path().join("test").exists(),
            "DeleteMatcher should actually delete files it matches",
        );

        let temp_dir_name = temp_dir.path().file_name().unwrap().to_string_lossy();
        let temp_dir_entry = get_dir_entry_for("test_data", &temp_dir_name);
        assert!(
            matcher.matches(&temp_dir_entry, &mut deps.new_matcher_io()),
            "DeleteMatcher should match directories",
        );
        assert!(
            !temp_dir.path().exists(),
            "DeleteMatcher should actually delete (empty) directories it matches",
        );
    }
}
