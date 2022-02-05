// Copyright 2017 Google Inc.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::error::Error;
use std::ffi::OsString;
use std::io::{stderr, Write};
use std::path::Path;
use std::process::Command;
use walkdir::DirEntry;

use super::{Matcher, MatcherIO};

enum Arg {
    Filename,
    LiteralArg(OsString),
}

pub struct SingleExecMatcher {
    executable: String,
    args: Vec<Arg>,
    exec_in_parent_dir: bool,
}

impl SingleExecMatcher {
    pub fn new(
        executable: &str,
        args: &[&str],
        exec_in_parent_dir: bool,
    ) -> Result<Self, Box<dyn Error>> {
        let transformed_args = args
            .iter()
            .map(|&a| match a {
                "{}" => Arg::Filename,
                _ => Arg::LiteralArg(OsString::from(a)),
            })
            .collect();

        Ok(Self {
            executable: executable.to_string(),
            args: transformed_args,
            exec_in_parent_dir,
        })
    }

    pub fn new_box(
        executable: &str,
        args: &[&str],
        exec_in_parent_dir: bool,
    ) -> Result<Box<dyn Matcher>, Box<dyn Error>> {
        Ok(Box::new(Self::new(executable, args, exec_in_parent_dir)?))
    }
}

impl Matcher for SingleExecMatcher {
    fn matches(&self, file_info: &DirEntry, _: &mut MatcherIO) -> bool {
        let mut command = Command::new(&self.executable);
        let path_to_file = if self.exec_in_parent_dir {
            if let Some(f) = file_info.path().file_name() {
                Path::new(".").join(f)
            } else {
                Path::new(".").join(file_info.path())
            }
        } else {
            file_info.path().to_path_buf()
        };

        for arg in &self.args {
            command.arg(match *arg {
                Arg::LiteralArg(ref a) => a.as_os_str(),
                Arg::Filename => path_to_file.as_os_str(),
            });
        }
        if self.exec_in_parent_dir {
            match file_info.path().parent() {
                None => {
                    // Root paths like "/" have no parent.  Run them from the root to match GNU find.
                    command.current_dir(file_info.path());
                }
                Some(parent) if parent == Path::new("") => {
                    // Paths like "foo" have a parent of "".  Avoid chdir("").
                }
                Some(parent) => {
                    command.current_dir(parent);
                }
            }
        }
        match command.status() {
            Ok(status) => status.success(),
            Err(e) => {
                writeln!(&mut stderr(), "Failed to run {}: {}", self.executable, e).unwrap();
                false
            }
        }
    }

    fn has_side_effects(&self) -> bool {
        true
    }
}

#[cfg(test)]
/// No tests here, because we need to call out to an external executable. See
/// tests/exec_unit_tests.rs instead.
mod tests {}
