/*
 * This file is part of the uutils findutils package.
 *
 * (c) Arcterus <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

use std::io::{self, stderr, Write};
use std::fs::{self, FileType};
use std::path::Path;

use walkdir::DirEntry;

use find::matchers::{Matcher, MatcherIO};

pub struct DeleteMatcher;

impl DeleteMatcher {
    pub fn new() -> DeleteMatcher {
        DeleteMatcher
    }

    pub fn new_box() -> Box<Matcher> {
        Box::new(DeleteMatcher::new())
    }

    fn delete(&self, file_path: &Path, file_type: FileType) -> io::Result<()> {
        if file_type.is_file() || file_type.is_symlink() {
            fs::remove_file(file_path)
        } else if file_type.is_dir() {
            fs::remove_dir(file_path)
        } else {
            unimplemented!() // TODO: not sure what find does for block devices, etc.
        }
    }
}

impl Matcher for DeleteMatcher {
    fn matches(&self, file_info: &DirEntry, _: &mut MatcherIO) -> bool {
        match self.delete(file_info.path(), file_info.file_type()) {
            Ok(_) => true,
            Err(f) => {
                writeln!(&mut stderr(),
                         "Failed to delete {}: {}",
                         file_info.path().to_string_lossy(),
                         f).unwrap();
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

}