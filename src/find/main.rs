// Copyright 2017 Google Inc.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::ffi::OsString;

/// Collect the process arguments as UTF-8 strings.
///
/// Unlike `std::env::args`, which panics on a non-UTF-8 argument, this reads the
/// arguments with `args_os` and reports the first invalid one as an error so the
/// caller can exit gracefully instead of aborting (#816).
fn collect_utf8_args(args: impl Iterator<Item = OsString>) -> Result<Vec<String>, OsString> {
    args.map(OsString::into_string).collect()
}

fn main() {
    // Ignores the SIGPIPE signal.
    // This is to solve the problem that when find is used with a pipe character,
    // the downstream software of the standard output stream closes the pipe and triggers a panic.
    uucore::panic::mute_sigpipe_panic();

    let args = match collect_utf8_args(std::env::args_os()) {
        Ok(args) => args,
        Err(invalid) => {
            eprintln!(
                "find: invalid (non-UTF-8) argument: {}",
                invalid.to_string_lossy()
            );
            std::process::exit(1);
        }
    };
    let strs: Vec<&str> = args.iter().map(std::convert::AsRef::as_ref).collect();
    let deps = findutils::find::StandardDependencies::new();
    std::process::exit(findutils::find::find_main(&strs, &deps));
}

#[cfg(test)]
mod tests {
    use super::collect_utf8_args;
    use std::ffi::OsString;

    #[test]
    fn collects_valid_utf8_args() {
        let args = [OsString::from("find"), OsString::from("-printf")];
        assert_eq!(
            collect_utf8_args(args.into_iter()).unwrap(),
            vec!["find".to_string(), "-printf".to_string()]
        );
    }

    #[test]
    #[cfg(unix)]
    fn non_utf8_arg_is_rejected_not_panicking() {
        use std::os::unix::ffi::OsStringExt;
        // A non-UTF-8 argument (e.g. `-printf $'%\xff'`) must produce an error
        // instead of panicking in std::env::args (#816).
        let args = [OsString::from("find"), OsString::from_vec(vec![b'%', 0xff])];
        assert!(collect_utf8_args(args.into_iter()).is_err());
    }
}
