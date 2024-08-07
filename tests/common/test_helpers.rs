// Copyright 2017 Google Inc.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::cell::RefCell;
use std::env;
use std::io::{Cursor, Read, Write};
use std::path::Path;
use std::time::SystemTime;

use findutils::find::matchers::{Follow, MatcherIO, WalkEntry};
use findutils::find::Dependencies;

/// A copy of `find::tests::FakeDependencies`.
/// TODO: find out how to share #[cfg(test)] functions/structs between unit
/// and integration tests.
pub struct FakeDependencies {
    pub output: RefCell<Cursor<Vec<u8>>>,
    now: SystemTime,
}

impl FakeDependencies {
    pub fn new() -> Self {
        Self {
            output: RefCell::new(Cursor::new(Vec::<u8>::new())),
            now: SystemTime::now(),
        }
    }

    pub fn new_matcher_io(&self) -> MatcherIO<'_> {
        MatcherIO::new(self)
    }

    pub fn get_output_as_string(&self) -> String {
        let mut cursor = self.output.borrow_mut();
        cursor.set_position(0);
        let mut contents = String::new();
        cursor.read_to_string(&mut contents).unwrap();
        contents
    }
}

impl Dependencies for FakeDependencies {
    fn get_output(&self) -> &RefCell<dyn Write> {
        &self.output
    }

    fn now(&self) -> SystemTime {
        self.now
    }
}

pub fn path_to_testing_commandline() -> String {
    let mut path_to_use = env::current_exe()
        // this will be something along the lines of /my/homedir/findutils/target/debug/deps/findutils-5532804878869ef1
        .expect("can't find path of this executable")
        .parent()
        .expect("can't find parent directory of this executable")
        .to_path_buf();
    // and we want /my/homedir/findutils/target/debug/testing-commandline
    if path_to_use.ends_with("deps") {
        path_to_use.pop();
    }
    path_to_use = path_to_use.join("testing-commandline");
    path_to_use.to_string_lossy().to_string()
}

#[cfg(windows)]
/// A copy of find::tests::fix_up_slashes.
/// TODO: find out how to share #[cfg(test)] functions/structs between unit
/// and integration tests.
pub fn fix_up_slashes(path: &str) -> String {
    path.replace("/", "\\")
}

#[cfg(not(windows))]
pub fn fix_up_slashes(path: &str) -> String {
    path.to_string()
}

/// A copy of `find::matchers::tests::get_dir_entry_for`.
/// TODO: find out how to share #[cfg(test)] functions/structs between unit
/// and integration tests.
pub fn get_dir_entry_for(root: &str, path: &str) -> WalkEntry {
    let root = fix_up_slashes(root);
    let root = Path::new(&root);

    let path = fix_up_slashes(path);
    let path = if path.is_empty() {
        root.to_owned()
    } else {
        root.join(path)
    };

    let depth = path.components().count() - root.components().count();

    WalkEntry::new(path, depth, Follow::Never)
}
