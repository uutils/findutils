// Copyright 2017 Google Inc.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::fs::File;
use std::io::{Write, stderr};

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;

use super::{Matcher, MatcherIO, StdoutErrors, WalkEntry};

pub enum PrintDelimiter {
    Newline,
    Null,
}

impl std::fmt::Display for PrintDelimiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Newline => writeln!(f),
            Self::Null => write!(f, "\0"),
        }
    }
}

/// This matcher just prints the name of the file to stdout.
pub struct Printer {
    delimiter: PrintDelimiter,
    output_file: Option<File>,
    stdout_errors: StdoutErrors,
}

impl Printer {
    pub fn new(delimiter: PrintDelimiter, output_file: Option<File>) -> Self {
        Self {
            delimiter,
            output_file,
            stdout_errors: StdoutErrors::default(),
        }
    }

    pub fn with_stdout_errors(mut self, stdout_errors: StdoutErrors) -> Self {
        self.stdout_errors = stdout_errors;
        self
    }

    fn print(&self, file_info: &WalkEntry, matcher_io: &mut MatcherIO, mut out: impl Write) {
        // Unix paths are byte strings: a lossy conversion can name a different file.
        #[cfg(unix)]
        let result = out.write_all(file_info.path().as_os_str().as_bytes());
        #[cfg(not(unix))]
        let result = write!(out, "{}", file_info.path().to_string_lossy());

        let result = result
            .and_then(|()| write!(out, "{}", self.delimiter))
            .and_then(|()| out.flush());
        if let Err(e) = result {
            if self.output_file.is_none() {
                self.stdout_errors.handle(&e, matcher_io);
                return;
            }
            if e.kind() == std::io::ErrorKind::BrokenPipe {
                std::process::exit(0);
            }
            writeln!(
                &mut stderr(),
                "Error writing {:?} for {}",
                file_info.path().to_string_lossy(),
                e
            )
            .unwrap();
            matcher_io.set_exit_code(1);
        }
    }
}

impl Matcher for Printer {
    fn matches(&self, file_info: &WalkEntry, matcher_io: &mut MatcherIO) -> bool {
        if let Some(file) = &self.output_file {
            self.print(file_info, matcher_io, file);
        } else {
            self.print(
                file_info,
                matcher_io,
                &mut *matcher_io.deps.get_output().borrow_mut(),
            );
        }
        true
    }

    fn has_side_effects(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::find::matchers::tests::get_dir_entry_for;
    use crate::find::tests::FakeDependencies;
    use crate::find::tests::fix_up_slashes;

    #[test]
    fn prints_newline() {
        let abbbc = get_dir_entry_for("./test_data/simple", "abbbc");

        let matcher = Printer::new(PrintDelimiter::Newline, None);
        let deps = FakeDependencies::new();
        assert!(matcher.matches(&abbbc, &mut deps.new_matcher_io()));
        assert_eq!(
            fix_up_slashes("./test_data/simple/abbbc\n"),
            deps.get_output_as_string()
        );
    }

    #[test]
    fn prints_null() {
        let abbbc = get_dir_entry_for("./test_data/simple", "abbbc");

        let matcher = Printer::new(PrintDelimiter::Null, None);
        let deps = FakeDependencies::new();
        assert!(matcher.matches(&abbbc, &mut deps.new_matcher_io()));
        assert_eq!(
            fix_up_slashes("./test_data/simple/abbbc\0"),
            deps.get_output_as_string()
        );
    }

    #[cfg(unix)]
    #[rstest::rstest]
    #[case::stdout_newline(PrintDelimiter::Newline, b"\n", false)]
    #[case::stdout_null(PrintDelimiter::Null, b"\0", false)]
    #[case::file_newline(PrintDelimiter::Newline, b"\n", true)]
    #[case::file_null(PrintDelimiter::Null, b"\0", true)]
    fn prints_raw_unix_path_bytes(
        #[case] delimiter: PrintDelimiter,
        #[case] suffix: &[u8],
        #[case] writes_file: bool,
    ) {
        use crate::find::matchers::Follow;
        use std::ffi::OsStr;
        use tempfile::Builder;

        // The input entry is synthetic because filesystems such as APFS reject
        // non-UTF-8 filenames. The output file has an ordinary name.
        let path = b"./caf\xc3\xa9\xff";
        let entry = WalkEntry::new(OsStr::from_bytes(path), 0, Follow::Never);
        let temp_dir = Builder::new()
            .prefix("prints_raw_unix_path_bytes_")
            .tempdir()
            .unwrap();
        let output_path = temp_dir.path().join("out");
        let output_file = if writes_file {
            Some(File::create(&output_path).unwrap())
        } else {
            None
        };
        let matcher = Printer::new(delimiter, output_file);
        let deps = FakeDependencies::new();
        assert!(matcher.matches(&entry, &mut deps.new_matcher_io()));
        let mut expected = path.to_vec();
        expected.extend_from_slice(suffix);
        if writes_file {
            assert_eq!(std::fs::read(output_path).unwrap(), expected);
            assert_eq!(deps.output.borrow().get_ref().as_slice(), []);
        } else {
            assert_eq!(deps.output.borrow().get_ref(), &expected);
        }
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn prints_error_message() {
        let dev_full = File::open("/dev/full").unwrap();
        let abbbc = get_dir_entry_for("./test_data/simple", "abbbc");

        let matcher = Printer::new(PrintDelimiter::Newline, Some(dev_full));
        let deps = FakeDependencies::new();

        assert!(matcher.matches(&abbbc, &mut deps.new_matcher_io()));
        assert_eq!(deps.get_output_as_string(), "");
    }
}
