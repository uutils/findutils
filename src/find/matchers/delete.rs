/*
 * This file is part of the uutils findutils package.
 *
 * (c) Arcterus <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

use std::env;
use std::fs::{self, FileType};
use std::io::{self, stderr, Write};
use std::path::{Path, PathBuf};

use walkdir::DirEntry;

use find::matchers::{Matcher, MatcherIO};

pub struct DeleteMatcher {
    current_dir: PathBuf,
}

impl DeleteMatcher {
    pub fn new() -> io::Result<DeleteMatcher> {
        Ok(DeleteMatcher {
            current_dir: env::current_dir()?,
        })
    }

    pub fn new_box() -> io::Result<Box<Matcher>> {
        Ok(Box::new(DeleteMatcher::new()?))
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
        if path == self.current_dir {
            return false;
        }

        match self.delete(path, file_info.file_type()) {
            Ok(_) => true,
            Err(f) => {
                writeln!(
                    &mut stderr(),
                    "Failed to delete {}: {}",
                    file_info.path().to_string_lossy(),
                    f
                )
                .unwrap();
                false
            }
        }
    }

    fn has_side_effects(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {}
