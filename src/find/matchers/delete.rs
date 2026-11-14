/*
 * This file is part of the uutils findutils package.
 *
 * (c) Arcterus <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

use std::fs;
use std::io::{self, stderr, ErrorKind, Write};

use super::{Matcher, MatcherIO, WalkEntry};

pub struct DeleteMatcher;

impl DeleteMatcher {
    pub fn new() -> Self {
        Self
    }

    fn delete(&self, entry: &WalkEntry) -> io::Result<()> {
        if entry.file_type().is_dir() && !entry.path_is_symlink() {
            fs::remove_dir(entry.path())
        } else {
            fs::remove_file(entry.path())
        }
    }
}

impl Matcher for DeleteMatcher {
    fn matches(&self, file_info: &WalkEntry, matcher_io: &mut MatcherIO) -> bool {
        let path = file_info.path();
        let path_str = path.to_string_lossy();

        // This is a quirk in find's traditional semantics probably due to
        // POSIX rmdir() not accepting "." (EINVAL). std::fs::remove_dir()
        // inherits the same behavior, so no reason to buck tradition.
        if path_str == "." {
            return true;
        }

        match self.delete(file_info) {
            Ok(()) => true,
            // With -ignore_readdir_race, a file that vanished since its
            // parent directory was read is not an error, and -delete is true.
            Err(e) if e.kind() == ErrorKind::NotFound && matcher_io.ignore_readdir_race() => true,
            Err(e) => {
                matcher_io.set_exit_code(1);
                writeln!(&mut stderr(), "Failed to delete {path_str}: {e}").unwrap();
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
    use std::fs::{create_dir, File};
    use tempfile::Builder;

    use super::*;
    use crate::find::matchers::tests::get_dir_entry_for;
    use crate::find::tests::FakeDependencies;

    #[test]
    fn delete_matcher() {
        let matcher = DeleteMatcher::new();
        let deps = FakeDependencies::new();

        let temp_dir = Builder::new().prefix("test_data").tempdir().unwrap();

        let temp_dir_path = temp_dir.path().to_string_lossy();
        File::create(temp_dir.path().join("test")).expect("created test file");
        create_dir(temp_dir.path().join("test_dir")).expect("created test directory");
        let test_entry = get_dir_entry_for(&temp_dir_path, "test");
        assert!(
            matcher.matches(&test_entry, &mut deps.new_matcher_io()),
            "DeleteMatcher should match a simple file",
        );
        assert!(
            !temp_dir.path().join("test").exists(),
            "DeleteMatcher should actually delete files it matches",
        );

        let temp_dir_entry = get_dir_entry_for(&temp_dir_path, "test_dir");
        assert!(
            matcher.matches(&temp_dir_entry, &mut deps.new_matcher_io()),
            "DeleteMatcher should match directories",
        );
        assert!(
            !temp_dir.path().join("test_dir").exists(),
            "DeleteMatcher should actually delete (empty) directories it matches",
        );
    }

    #[test]
    fn delete_matcher_vanished_file() {
        let matcher = DeleteMatcher::new();
        let deps = FakeDependencies::new();

        let temp_dir = Builder::new().prefix("test_data").tempdir().unwrap();
        let temp_dir_path = temp_dir.path().to_string_lossy();
        let test_file = temp_dir.path().join("test");
        File::create(&test_file).expect("created test file");
        let test_entry = get_dir_entry_for(&temp_dir_path, "test");

        // The file disappears after its entry was read from the directory.
        fs::remove_file(&test_file).expect("removed test file");

        let mut matcher_io = deps.new_matcher_io();
        assert!(
            !matcher.matches(&test_entry, &mut matcher_io),
            "DeleteMatcher should fail on a vanished file by default",
        );
        assert_eq!(matcher_io.exit_code(), 1);

        let mut matcher_io = deps.new_matcher_io();
        matcher_io.set_ignore_readdir_race(true);
        assert!(
            matcher.matches(&test_entry, &mut matcher_io),
            "DeleteMatcher should be true for a vanished file with -ignore_readdir_race",
        );
        assert_eq!(matcher_io.exit_code(), 0);
    }
}
