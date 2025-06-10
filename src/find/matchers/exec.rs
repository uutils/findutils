// Copyright 2017 Google Inc.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::cell::RefCell;
use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::io::{stderr, Write};
use std::path::Path;
use std::process::Command;

use super::{Matcher, MatcherIO, WalkEntry};

fn check_path_entries_absolute(path: Option<OsString>) -> Result<(), Box<dyn Error>> {
    if let Some(path_dirs) = path {
        for dir_entry in env::split_paths(&path_dirs) {
            if !dir_entry.is_absolute() || dir_entry.as_os_str().is_empty() {
                return Err(format!(
                    "The PATH environment variable contains non-absolute paths or empty paths. \
                    Segment that caused the error: '{}'",
                    dir_entry.as_os_str().to_string_lossy()
                )
                .into());
            }
        }
    }

    Ok(())
}

#[derive(Debug)]
enum Arg {
    FileArg(Vec<OsString>),
    LiteralArg(OsString),
}

#[derive(Debug)]
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
        Self::new_with_path(executable, args, exec_in_parent_dir, env::var_os("PATH"))
    }

    fn new_with_path(
        executable: &str,
        args: &[&str],
        exec_in_parent_dir: bool,
        path: Option<OsString>,
    ) -> Result<Self, Box<dyn Error>> {
        if exec_in_parent_dir {
            check_path_entries_absolute(path)?;
        }

        let transformed_args = args
            .iter()
            .map(|&a| {
                let parts = a.split("{}").collect::<Vec<_>>();
                if parts.len() == 1 {
                    // No {} present
                    Arg::LiteralArg(OsString::from(a))
                } else {
                    Arg::FileArg(parts.iter().map(OsString::from).collect())
                }
            })
            .collect();

        Ok(Self {
            executable: executable.to_string(),
            args: transformed_args,
            exec_in_parent_dir,
        })
    }
}

impl Matcher for SingleExecMatcher {
    fn matches(&self, file_info: &WalkEntry, _: &mut MatcherIO) -> bool {
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
            match *arg {
                Arg::LiteralArg(ref a) => command.arg(a.as_os_str()),
                Arg::FileArg(ref parts) => command.arg(parts.join(path_to_file.as_os_str())),
            };
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

#[derive(Debug)]
pub struct MultiExecMatcher {
    executable: String,
    args: Vec<OsString>,
    exec_in_parent_dir: bool,
    /// Command to build while matching.
    command: RefCell<Option<argmax::Command>>,
}

impl MultiExecMatcher {
    pub fn new(
        executable: &str,
        args: &[&str],
        exec_in_parent_dir: bool,
    ) -> Result<Self, Box<dyn Error>> {
        Self::new_with_path(executable, args, exec_in_parent_dir, env::var_os("PATH"))
    }

    fn new_with_path(
        executable: &str,
        args: &[&str],
        exec_in_parent_dir: bool,
        path: Option<OsString>,
    ) -> Result<Self, Box<dyn Error>> {
        if exec_in_parent_dir {
            check_path_entries_absolute(path)?;
        }

        let transformed_args = args.iter().map(OsString::from).collect();

        Ok(Self {
            executable: executable.to_string(),
            args: transformed_args,
            exec_in_parent_dir,
            command: RefCell::new(None),
        })
    }

    fn new_command(&self) -> argmax::Command {
        let mut command = argmax::Command::new(&self.executable);
        command.try_args(&self.args).unwrap();
        command
    }

    fn run_command(&self, command: &mut argmax::Command, matcher_io: &mut MatcherIO) {
        match command.status() {
            Ok(status) => {
                if !status.success() {
                    matcher_io.set_exit_code(1);
                }
            }
            Err(e) => {
                writeln!(&mut stderr(), "Failed to run {}: {}", self.executable, e).unwrap();
                matcher_io.set_exit_code(1);
            }
        }
    }
}

impl Matcher for MultiExecMatcher {
    fn matches(&self, file_info: &WalkEntry, matcher_io: &mut MatcherIO) -> bool {
        let path_to_file = if self.exec_in_parent_dir {
            if let Some(f) = file_info.path().file_name() {
                Path::new(".").join(f)
            } else {
                Path::new(".").join(file_info.path())
            }
        } else {
            file_info.path().to_path_buf()
        };
        let mut command = self.command.borrow_mut();
        let command = command.get_or_insert_with(|| self.new_command());

        // Build command, or dispatch it before when it is long enough.
        if command.try_arg(&path_to_file).is_err() {
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
            self.run_command(command, matcher_io);

            // Reset command status.
            *command = self.new_command();
            if let Err(e) = command.try_arg(&path_to_file) {
                writeln!(
                    &mut stderr(),
                    "Cannot fit a single argument {}: {}",
                    &path_to_file.to_string_lossy(),
                    e
                )
                .unwrap();
                matcher_io.set_exit_code(1);
            }
        }
        true
    }

    fn finished_dir(&self, dir: &Path, matcher_io: &mut MatcherIO) {
        // Dispatch command for -execdir.
        if self.exec_in_parent_dir {
            let mut command = self.command.borrow_mut();
            if let Some(mut command) = command.take() {
                command.current_dir(Path::new(".").join(dir));
                self.run_command(&mut command, matcher_io);
            }
        }
    }

    fn finished(&self, matcher_io: &mut MatcherIO) {
        // Dispatch command for -exec.
        if !self.exec_in_parent_dir {
            let mut command = self.command.borrow_mut();
            if let Some(mut command) = command.take() {
                self.run_command(&mut command, matcher_io);
            }
        }
    }

    fn has_side_effects(&self) -> bool {
        true
    }
}

/// Only tests related to path checking here, because we need to call out to an external executable.
/// See `tests/exec_unit_tests.rs` instead.
#[cfg(test)]
mod check_path_tests {
    use super::*;
    use std::path::MAIN_SEPARATOR;

    #[cfg(unix)]
    use std::os::unix::ffi::OsStringExt;
    #[cfg(unix)]
    const PATH_SEPARATOR: char = ':';
    #[cfg(windows)]
    const PATH_SEPARATOR: char = ';';
    #[cfg(windows)]
    use std::os::windows::ffi::OsStringExt;

    // Helper to create platform-specific absolute paths
    fn abs_path(component: &str) -> String {
        format!("{}{}", MAIN_SEPARATOR, component)
    }

    // Helper to create platform-specific relative paths
    fn rel_path(component: &str) -> String {
        format!(".{}{}", MAIN_SEPARATOR, component)
    }

    mod single_exec_matcher_tests {
        use super::*;

        #[test]
        fn single_exec_matcher_valid_path() {
            #[cfg(unix)]
            let path = format!("{}{}{}", abs_path("usr"), PATH_SEPARATOR, abs_path("bin"));
            #[cfg(windows)]
            let path = format!(
                "C:{}{}C:{}",
                abs_path("usr"),
                PATH_SEPARATOR,
                abs_path("bin")
            );
            let result = SingleExecMatcher::new_with_path(
                "echo",
                &["test"],
                true,
                Some(OsString::from(path)),
            );
            assert!(result.is_ok());
        }

        #[test]
        fn single_exec_matcher_multiple_valid_paths() {
            #[cfg(unix)]
            let path = format!(
                "{}{}{}{}{}",
                abs_path("a"),
                PATH_SEPARATOR,
                abs_path("b"),
                PATH_SEPARATOR,
                abs_path("c")
            );
            #[cfg(windows)]
            let path = format!(
                "C:{}{}C:{}{}C:{}",
                abs_path("a"),
                PATH_SEPARATOR,
                abs_path("b"),
                PATH_SEPARATOR,
                abs_path("c")
            );
            let result = SingleExecMatcher::new_with_path(
                "echo",
                &["test"],
                true,
                Some(OsString::from(path)),
            );
            assert!(result.is_ok());
        }

        #[test]
        fn single_exec_matcher_relative_path() {
            let path = format!(
                "{}{}{}",
                abs_path("usr"),
                PATH_SEPARATOR,
                rel_path("relative")
            );
            let result = SingleExecMatcher::new_with_path(
                "echo",
                &["test"],
                true,
                Some(OsString::from(path)),
            );
            assert!(result.is_err());
        }

        #[test]
        fn single_exec_matcher_empty_path() {
            let path = format!("{}{}{}", abs_path("usr"), PATH_SEPARATOR, "");
            let result = SingleExecMatcher::new_with_path(
                "echo",
                &["test"],
                true,
                Some(OsString::from(path)),
            );
            assert!(result.is_err());
        }

        #[test]
        fn single_exec_matcher_empty_string_path() {
            let result =
                SingleExecMatcher::new_with_path("echo", &["test"], true, Some(OsString::from("")));
            assert!(result.is_err());
        }

        #[test]
        fn single_exec_matcher_valid_then_invalid_path() {
            let path = format!(
                "{}{}{}{}{}",
                abs_path("valid1"),
                PATH_SEPARATOR,
                rel_path("invalid"),
                PATH_SEPARATOR,
                abs_path("valid2")
            );
            let result = SingleExecMatcher::new_with_path(
                "echo",
                &["test"],
                true,
                Some(OsString::from(path)),
            );
            assert!(result.is_err());
        }

        #[test]
        fn single_exec_matcher_undefined_path() {
            let result = SingleExecMatcher::new_with_path("echo", &["test"], true, None);
            assert!(result.is_ok());
        }

        #[test]
        fn single_exec_matcher_no_validation_when_not_needed() {
            let path = format!("{}{}{}", "relative", PATH_SEPARATOR, "");
            let result = SingleExecMatcher::new_with_path(
                "echo",
                &["test"],
                false,
                Some(OsString::from(path)),
            );
            assert!(result.is_ok());
        }

        #[test]
        fn single_exec_matcher_relative_path_error_message() {
            let relative_component = rel_path("relative");
            #[cfg(unix)]
            let path = format!(
                "{}{}{}",
                abs_path("usr"),
                PATH_SEPARATOR,
                relative_component
            );
            #[cfg(windows)]
            let path = format!(
                "C:{}{}C:{}",
                abs_path("usr"),
                PATH_SEPARATOR,
                relative_component
            );
            let result = SingleExecMatcher::new_with_path(
                "echo",
                &["test"],
                true,
                Some(OsString::from(path)),
            );
            let err = result.unwrap_err();
            let err_msg = err.to_string();
            assert!(
                err_msg.contains(&relative_component),
                "Error message should contain relative path component"
            );
        }

        #[test]
        fn single_exec_matcher_empty_path_error_message() {
            #[cfg(unix)]
            let path = format!("{}{}{}", abs_path("usr"), PATH_SEPARATOR, "");
            #[cfg(windows)]
            let path = format!("C:{}{}{}", abs_path("usr"), PATH_SEPARATOR, "");
            let result = SingleExecMatcher::new_with_path(
                "echo",
                &["test"],
                true,
                Some(OsString::from(path)),
            );
            let err = result.unwrap_err();
            let err_msg = err.to_string();
            assert!(
                err_msg.contains("''"),
                "Error message should contain empty path indicator"
            );
        }

        // Platform-specific non-UTF8 tests
        #[cfg(unix)]
        #[test]
        #[ignore]
        fn single_exec_matcher_does_not_reject_non_utf8_unix() {
            let result = SingleExecMatcher::new_with_path(
                "echo",
                &["test"],
                true,
                Some(OsString::from_vec(vec![
                    b'\x2F', b'\x00', b'\x75', b'\x00', b'\x73', b'\x00', b'\x72', b'\x00',
                    b'\x2F', b'\x00', b'\x62', b'\x00', b'\x69', b'\x00', b'\x6E', b'\x00',
                    b'\x3A', b'\x00', b'\x2F', b'\x00', b'\x62', b'\x00', b'\x69', b'\x00',
                    b'\x6E', b'\x00',
                ])),
            );
            assert!(result.is_ok());
        }

        #[cfg(windows)]
        #[test]
        #[ignore]
        fn single_exec_matcher_does_not_reject_non_utf8_windows() {
            let result = SingleExecMatcher::new_with_path(
                "echo",
                &["test"],
                true,
                Some(OsString::from_wide(&[
                    0x0043, 0x003A, 0x005C, 0x0076, 0x0061, 0x006C, 0x0069, 0x0064, 0x003B, 0xD800,
                    0xDC00,
                ])),
            );
            assert!(result.is_ok());
        }
    }

    mod multi_exec_matcher_test {
        use super::*;

        // Tests mirroring the single_exec tests with MultiExecMatcher
        // (Same test structure as above but using MultiExecMatcher)
        #[test]
        fn multi_exec_matcher_valid_path() {
            #[cfg(unix)]
            let path = format!("{}{}{}", abs_path("usr"), PATH_SEPARATOR, abs_path("bin"));
            #[cfg(windows)]
            let path = format!(
                "C:{}{}C:{}",
                abs_path("usr"),
                PATH_SEPARATOR,
                abs_path("bin")
            );

            let result = MultiExecMatcher::new_with_path(
                "echo",
                &["test"],
                true,
                Some(OsString::from(path)),
            );
            assert!(result.is_ok());
        }

        #[test]
        fn multi_exec_matcher_multiple_valid_paths() {
            #[cfg(unix)]
            let path = format!(
                "{}{}{}{}{}",
                abs_path("a"),
                PATH_SEPARATOR,
                abs_path("b"),
                PATH_SEPARATOR,
                abs_path("c")
            );
            #[cfg(windows)]
            let path = format!(
                "C:{}{}C:{}{}C:{}",
                abs_path("a"),
                PATH_SEPARATOR,
                abs_path("b"),
                PATH_SEPARATOR,
                abs_path("c")
            );
            let result = MultiExecMatcher::new_with_path(
                "echo",
                &["test"],
                true,
                Some(OsString::from(path.clone())),
            );

            assert!(result.is_ok());
        }

        #[test]
        fn multi_exec_matcher_relative_path() {
            let path = format!(
                "{}{}{}",
                abs_path("usr"),
                PATH_SEPARATOR,
                rel_path("relative")
            );
            let result = MultiExecMatcher::new_with_path(
                "echo",
                &["test"],
                true,
                Some(OsString::from(path)),
            );
            assert!(result.is_err());
        }

        #[test]
        fn multi_exec_matcher_empty_path() {
            let path = format!("{}{}{}", abs_path("usr"), PATH_SEPARATOR, "");
            let result = MultiExecMatcher::new_with_path(
                "echo",
                &["test"],
                true,
                Some(OsString::from(path)),
            );
            assert!(result.is_err());
        }

        #[test]
        fn multi_exec_matcher_empty_string_path() {
            let result =
                MultiExecMatcher::new_with_path("echo", &["test"], true, Some(OsString::from("")));
            assert!(result.is_err());
        }

        #[test]
        fn multi_exec_matcher_valid_then_invalid_path() {
            let path = format!(
                "{}{}{}{}{}",
                abs_path("valid1"),
                PATH_SEPARATOR,
                rel_path("invalid"),
                PATH_SEPARATOR,
                abs_path("valid2")
            );
            let result = MultiExecMatcher::new_with_path(
                "echo",
                &["test"],
                true,
                Some(OsString::from(path)),
            );
            assert!(result.is_err());
        }

        #[test]
        fn multi_exec_matcher_undefined_path() {
            let result = MultiExecMatcher::new_with_path("echo", &["test"], true, None);
            assert!(result.is_ok());
        }

        #[test]
        fn multi_exec_matcher_no_validation_when_not_needed() {
            let path = format!("{}{}{}", "relative", PATH_SEPARATOR, "");
            let result = MultiExecMatcher::new_with_path(
                "echo",
                &["test"],
                false,
                Some(OsString::from(path)),
            );
            assert!(result.is_ok());
        }

        // Platform-specific non-UTF8 tests
        #[cfg(unix)]
        #[test]
        #[ignore]
        fn multi_exec_matcher_does_not_reject_non_utf8_unix() {
            let result = MultiExecMatcher::new_with_path(
                "echo",
                &["test"],
                true,
                Some(OsString::from_vec(vec![
                    b'\x2F', b'\x00', b'\x75', b'\x00', b'\x73', b'\x00', b'\x72', b'\x00',
                    b'\x2F', b'\x00', b'\x62', b'\x00', b'\x69', b'\x00', b'\x6E', b'\x00',
                    b'\x3A', b'\x00', b'\x2F', b'\x00', b'\x62', b'\x00', b'\x69', b'\x00',
                    b'\x6E', b'\x00',
                ])),
            );
            assert!(result.is_ok());
        }

        #[cfg(windows)]
        #[test]
        #[ignore]
        fn multi_exec_matcher_does_not_reject_non_utf8_windows() {
            let result = MultiExecMatcher::new_with_path(
                "echo",
                &["test"],
                true,
                Some(OsString::from_wide(&[
                    0x0043, 0x003A, 0x005C, 0x0076, 0x0061, 0x006C, 0x0069, 0x0064, 0x003B, 0xD800,
                    0xDC00,
                ])),
            );
            assert!(result.is_ok());
        }
    }
}
