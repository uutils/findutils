// Copyright 2017 Google Inc.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/// ! This file contains what would be normally be unit tests for `find::matchers::exec`.
/// ! But as the tests require running an external executable, they need to be run
/// ! as integration tests so we can ensure that our testing-commandline binary
/// ! has been built.
use serial_test::serial;
use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use tempfile::{tempdir, Builder};

use common::test_helpers::{
    fix_up_slashes, get_dir_entry_for, path_to_testing_commandline, FakeDependencies,
};
use findutils::find::matchers::exec::{MultiExecMatcher, SingleExecMatcher};
use findutils::find::matchers::{Matcher, MatcherIO};

mod common;

#[test]
#[serial(path_addicted)]
fn matching_executes_code() {
    let temp_dir = Builder::new()
        .prefix("matching_executes_code")
        .tempdir()
        .unwrap();
    let temp_dir_path = temp_dir.path().to_string_lossy();

    let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
    let matcher = SingleExecMatcher::new(
        &path_to_testing_commandline(),
        &[temp_dir_path.as_ref(), "abc", "{}", "xyz"],
        false,
    )
    .expect("Failed to create matcher");
    let deps = FakeDependencies::new();
    assert!(matcher.matches(&abbbc, &mut deps.new_matcher_io()));

    let mut f = File::open(temp_dir.path().join("1.txt")).expect("Failed to open output file");
    let mut s = String::new();
    f.read_to_string(&mut s)
        .expect("failed to read output file");
    assert_eq!(
        s,
        fix_up_slashes(&format!(
            "cwd={}\nargs=\nabc\ntest_data/simple/abbbc\nxyz\n",
            env::current_dir().unwrap().to_string_lossy()
        ))
    );
}

#[test]
#[serial(path_addicted)]
fn matching_executes_code_in_files_directory() {
    let temp_dir = Builder::new()
        .prefix("matching_executes_code_in_files_directory")
        .tempdir()
        .unwrap();
    let temp_dir_path = temp_dir.path().to_string_lossy();

    let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
    let matcher = SingleExecMatcher::new(
        &path_to_testing_commandline(),
        &[temp_dir_path.as_ref(), "abc", "{}", "xyz"],
        true,
    )
    .expect("Failed to create matcher");
    let deps = FakeDependencies::new();
    assert!(matcher.matches(&abbbc, &mut deps.new_matcher_io()));

    let mut f = File::open(temp_dir.path().join("1.txt")).expect("Failed to open output file");
    let mut s = String::new();
    f.read_to_string(&mut s)
        .expect("failed to read output file");
    assert_eq!(
        s,
        fix_up_slashes(&format!(
            "cwd={}/test_data/simple\nargs=\nabc\n./abbbc\nxyz\n",
            env::current_dir().unwrap().to_string_lossy()
        ))
    );
}

#[test]
#[serial(path_addicted)]
fn matching_embedded_filename() {
    let temp_dir = Builder::new()
        .prefix("matching_embedded_filename")
        .tempdir()
        .unwrap();
    let temp_dir_path = temp_dir.path().to_string_lossy();

    let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
    let matcher = SingleExecMatcher::new(
        &path_to_testing_commandline(),
        &[temp_dir_path.as_ref(), "abc{}x{}yz"],
        false,
    )
    .expect("Failed to create matcher");
    let deps = FakeDependencies::new();
    assert!(matcher.matches(&abbbc, &mut deps.new_matcher_io()));

    let mut f = File::open(temp_dir.path().join("1.txt")).expect("Failed to open output file");
    let mut s = String::new();
    f.read_to_string(&mut s)
        .expect("failed to read output file");
    assert_eq!(
        s,
        fix_up_slashes(&format!(
            "cwd={}\nargs=\nabctest_data/simple/abbbcxtest_data/simple/abbbcyz\n",
            env::current_dir().unwrap().to_string_lossy()
        ))
    );
}

#[test]
#[serial(path_addicted)]
/// Running "find . -execdir whatever \;" failed with a No such file or directory error.
/// It's now fixed, and this is a regression test to check that it stays fixed.
fn execdir_in_current_directory() {
    let temp_dir = Builder::new()
        .prefix("execdir_in_current_directory")
        .tempdir()
        .unwrap();
    let temp_dir_path = temp_dir.path().to_string_lossy();

    let current_dir_entry = get_dir_entry_for(".", "");
    let matcher = SingleExecMatcher::new(
        &path_to_testing_commandline(),
        &[temp_dir_path.as_ref(), "abc", "{}", "xyz"],
        true,
    )
    .expect("Failed to create matcher");
    let deps = FakeDependencies::new();
    assert!(matcher.matches(&current_dir_entry, &mut deps.new_matcher_io()));

    let mut f = File::open(temp_dir.path().join("1.txt")).expect("Failed to open output file");
    let mut s = String::new();
    f.read_to_string(&mut s)
        .expect("failed to read output file");
    assert_eq!(
        s,
        fix_up_slashes(&format!(
            "cwd={}\nargs=\nabc\n./.\nxyz\n",
            env::current_dir().unwrap().to_string_lossy()
        ))
    );
}

#[test]
#[serial(path_addicted)]
/// Regression test for "find / -execdir whatever \;"
fn execdir_in_root_directory() {
    let temp_dir = Builder::new()
        .prefix("execdir_in_root_directory")
        .tempdir()
        .unwrap();
    let temp_dir_path = temp_dir.path().to_string_lossy();

    let cwd = env::current_dir().expect("no current directory");
    let root_dir = cwd
        .ancestors()
        .last()
        .expect("current directory has no root");
    let root_dir_entry = get_dir_entry_for(root_dir.to_str().unwrap(), "");

    let matcher = SingleExecMatcher::new(
        &path_to_testing_commandline(),
        &[temp_dir_path.as_ref(), "abc", "{}", "xyz"],
        true,
    )
    .expect("Failed to create matcher");
    let deps = FakeDependencies::new();
    assert!(matcher.matches(&root_dir_entry, &mut deps.new_matcher_io()));

    let mut f = File::open(temp_dir.path().join("1.txt")).expect("Failed to open output file");
    let mut s = String::new();
    f.read_to_string(&mut s)
        .expect("failed to read output file");
    assert_eq!(
        s,
        fix_up_slashes(&format!(
            "cwd={}\nargs=\nabc\n{}\nxyz\n",
            root_dir.to_string_lossy(),
            root_dir.to_string_lossy(),
        ))
    );
}

#[test]
#[serial(path_addicted)]
fn matching_fails_if_executable_fails() {
    let temp_dir = Builder::new()
        .prefix("matching_fails_if_executable_fails")
        .tempdir()
        .unwrap();
    let temp_dir_path = temp_dir.path().to_string_lossy();

    let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
    let matcher = SingleExecMatcher::new(
        &path_to_testing_commandline(),
        &[
            temp_dir_path.as_ref(),
            "--exit_with_failure",
            "abc",
            "{}",
            "xyz",
        ],
        true,
    )
    .expect("Failed to create matcher");
    let deps = FakeDependencies::new();
    assert!(!matcher.matches(&abbbc, &mut deps.new_matcher_io()));

    let mut f = File::open(temp_dir.path().join("1.txt")).expect("Failed to open output file");
    let mut s = String::new();
    f.read_to_string(&mut s)
        .expect("failed to read output file");
    assert_eq!(
        s,
        fix_up_slashes(&format!(
            "cwd={}/test_data/simple\nargs=\n--exit_with_failure\nabc\n.\
             /abbbc\nxyz\n",
            env::current_dir().unwrap().to_string_lossy()
        ))
    );
}

#[test]
#[serial(path_addicted)]
fn matching_multi_executes_code() {
    let temp_dir = Builder::new()
        .prefix("matching_executes_code")
        .tempdir()
        .unwrap();
    let temp_dir_path = temp_dir.path().to_string_lossy();

    let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
    let matcher = MultiExecMatcher::new(
        &path_to_testing_commandline(),
        &[temp_dir_path.as_ref(), "abc"],
        false,
    )
    .expect("Failed to create matcher");
    let deps = FakeDependencies::new();
    let mut matcher_io = MatcherIO::new(&deps);
    assert!(matcher.matches(&abbbc, &mut deps.new_matcher_io()));
    matcher.finished(&mut matcher_io);

    let mut f = File::open(temp_dir.path().join("1.txt")).expect("Failed to open output file");
    let mut s = String::new();
    f.read_to_string(&mut s)
        .expect("failed to read output file");
    assert_eq!(
        s,
        fix_up_slashes(&format!(
            "cwd={}\nargs=\nabc\ntest_data/simple/abbbc\n",
            env::current_dir().unwrap().to_string_lossy()
        ))
    );
}

#[test]
#[serial(path_addicted)]
fn execdir_multi_in_current_directory() {
    let temp_dir = Builder::new()
        .prefix("execdir_in_current_directory")
        .tempdir()
        .unwrap();
    let temp_dir_path = temp_dir.path().to_string_lossy();

    let current_dir_entry = get_dir_entry_for(".", "");
    let matcher = MultiExecMatcher::new(
        &path_to_testing_commandline(),
        &[temp_dir_path.as_ref(), "abc"],
        true,
    )
    .expect("Failed to create matcher");
    let deps = FakeDependencies::new();
    let mut matcher_io = MatcherIO::new(&deps);
    assert!(matcher.matches(&current_dir_entry, &mut deps.new_matcher_io()));
    matcher.finished_dir(Path::new(""), &mut matcher_io);
    matcher.finished(&mut matcher_io);

    let mut f = File::open(temp_dir.path().join("1.txt")).expect("Failed to open output file");
    let mut s = String::new();
    f.read_to_string(&mut s)
        .expect("failed to read output file");
    assert_eq!(
        s,
        fix_up_slashes(&format!(
            "cwd={}\nargs=\nabc\n./.\n",
            env::current_dir().unwrap().to_string_lossy()
        ))
    );
}

#[test]
#[serial(path_addicted)]
fn multi_set_exit_code_if_executable_fails() {
    let temp_dir = Builder::new()
        .prefix("multi_set_exit_code_if_executable_fails")
        .tempdir()
        .unwrap();
    let temp_dir_path = temp_dir.path().to_string_lossy();

    let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
    let matcher = MultiExecMatcher::new(
        &path_to_testing_commandline(),
        &[temp_dir_path.as_ref(), "--exit_with_failure", "abc"],
        true,
    )
    .expect("Failed to create matcher");
    let deps = FakeDependencies::new();
    assert!(matcher.matches(&abbbc, &mut deps.new_matcher_io()));
    let mut matcher_io = deps.new_matcher_io();
    matcher.finished_dir(Path::new("test_data/simple"), &mut matcher_io);
    assert!(matcher_io.exit_code() == 1);

    let mut f = File::open(temp_dir.path().join("1.txt")).expect("Failed to open output file");
    let mut s = String::new();
    f.read_to_string(&mut s)
        .expect("failed to read output file");
    assert_eq!(
        s,
        fix_up_slashes(&format!(
            "cwd={}/test_data/simple\nargs=\n--exit_with_failure\nabc\n./abbbc\n",
            env::current_dir().unwrap().to_string_lossy()
        ))
    );
}

#[test]
#[serial(path_addicted)]
fn multi_set_exit_code_if_command_fails() {
    let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
    let matcher = MultiExecMatcher::new("1337", &["abc"], true).expect("Failed to create matcher");
    let deps = FakeDependencies::new();
    assert!(matcher.matches(&abbbc, &mut deps.new_matcher_io()));
    let mut matcher_io = deps.new_matcher_io();
    matcher.finished_dir(Path::new("test_data/simple"), &mut matcher_io);
    assert!(matcher_io.exit_code() == 1);
}

// -- tests for checking path integrity
// SAFETY:
// The only use of unsafe was for the std::env::set_var function.
// Safety is guaranteed because all tests that directly depend
// on the PATH variable were marked with the macro #[serial(path_addicted)],
// making the execution of this serial and avoiding concurrency
// and data-race problems. Furthermore, any test that somehow
// changes the PATH, immediately after execution returns the
// PATH to the original value of the variable before the change.

#[test]
#[serial(path_addicted)]
fn test_empty_path() {
    let original_path = env::var("PATH").unwrap();
    let (valid_segment1, valid_segment2, valid_segment3) = get_valid_segments();
    let separator = get_path_separator();

    let invalid_path = format!(
        "{}{}{}{}{}{}{}",
        valid_segment1, separator, valid_segment2, separator, valid_segment3, separator, separator
    );

    unsafe {
        env::set_var("PATH", invalid_path);
    }
    let matcher = MultiExecMatcher::new("1337", &["abc"], true);
    unsafe {
        env::set_var("PATH", original_path);
    }

    assert!(matcher.is_err());
}

#[test]
#[serial(path_addicted)]
fn test_non_absolute_path() {
    let original_path = env::var("PATH").unwrap();
    let (valid_segment1, valid_segment2, valid_segment3) = get_valid_segments();
    let separator = get_path_separator();

    let invalid_path = format!(
        "{}{}{}{}{}{}relative{}valid_segment4",
        valid_segment1, separator, valid_segment2, separator, valid_segment3, separator, separator
    );

    unsafe {
        env::set_var("PATH", invalid_path);
    }
    let matcher = MultiExecMatcher::new("1337", &["abc"], true);
    unsafe {
        env::set_var("PATH", original_path);
    }

    assert!(matcher.is_err());
}

#[test]
#[serial(path_addicted)]
fn test_file_path() {
    let temp_dir = tempdir().expect("Failed to create temp file");
    let temp_dir_path = temp_dir.path();
    let absolute_file_path = temp_dir_path.join("__test_temp_file");
    File::create(absolute_file_path.clone())
        .expect("Failed to create file")
        .write_all(b"foo")
        .expect("Failed to write to file");

    let original_path = env::var("PATH").unwrap();
    let (valid_segment1, valid_segment2, valid_segment3) = get_valid_segments();
    let separator = get_path_separator();

    let invalid_path = format!(
        "{}{}{}{}{}{}{}",
        valid_segment1,
        separator,
        valid_segment2,
        separator,
        valid_segment3,
        separator,
        absolute_file_path.to_str().unwrap()
    );

    unsafe {
        env::set_var("PATH", invalid_path);
    }
    let matcher = MultiExecMatcher::new("1337", &["abc"], true);
    unsafe {
        env::set_var("PATH", original_path);
    }

    assert!(matcher.is_err());
}

#[test]
#[serial(path_addicted)]
fn valid_path_single_exec() {
    let original_path = env::var("PATH").unwrap();
    let (valid_segment1, valid_segment2, valid_segment3) = get_valid_segments();
    let separator = get_path_separator();

    let valid_path = format!(
        "{}{}{}{}{}",
        valid_segment1, separator, valid_segment2, separator, valid_segment3
    );

    unsafe {
        env::set_var("PATH", valid_path);
    }
    let matcher = SingleExecMatcher::new("true", &[], true);
    unsafe {
        env::set_var("PATH", original_path);
    }

    assert!(matcher.is_ok());
}

#[test]
#[serial(path_addicted)]
fn valid_path_multi_exec() {
    let original_path = env::var("PATH").unwrap();
    let (valid_segment1, valid_segment2, valid_segment3) = get_valid_segments();
    let separator = get_path_separator();

    let valid_path = format!(
        "{}{}{}{}{}",
        valid_segment1, separator, valid_segment2, separator, valid_segment3
    );

    unsafe {
        env::set_var("PATH", valid_path);
    }
    let matcher = MultiExecMatcher::new("true", &[], true);
    unsafe {
        env::set_var("PATH", original_path);
    }

    assert!(matcher.is_ok());
}

#[test]
#[serial(path_addicted)]
fn empty_path_segment_single_exec() {
    let original_path = env::var("PATH").unwrap();
    let (valid_segment1, valid_segment2, valid_segment3) = get_valid_segments();
    let separator = get_path_separator();

    let invalid_path = format!(
        "{}{}{}{}{}{}{}",
        valid_segment1, separator, valid_segment2, separator, valid_segment3, separator, separator
    );

    unsafe {
        env::set_var("PATH", invalid_path);
    }
    let matcher = SingleExecMatcher::new("true", &[], true);
    unsafe {
        env::set_var("PATH", original_path);
    }

    assert!(matcher.is_err());
}

#[test]
#[serial(path_addicted)]
fn relative_path_segment_single_exec() {
    let original_path = env::var("PATH").unwrap();
    let (valid_segment1, valid_segment2, valid_segment3) = get_valid_segments();
    let separator = get_path_separator();

    let invalid_path = format!(
        "{}{}{}{}{}{}relative{}valid_segment4",
        valid_segment1, separator, valid_segment2, separator, valid_segment3, separator, separator
    );

    unsafe {
        env::set_var("PATH", invalid_path);
    }
    let matcher = SingleExecMatcher::new("true", &[], true);
    unsafe {
        env::set_var("PATH", original_path);
    }

    assert!(matcher.is_err());
}

#[test]
#[serial(path_addicted)]
fn file_path_segment_single_exec() {
    let temp_dir = tempdir().expect("Failed to create temp file");
    let temp_dir_path = temp_dir.path();
    let absolute_file_path = temp_dir_path.join("test_file");

    File::create(&absolute_file_path)
        .expect("Failed to create file")
        .write_all(b"foo")
        .expect("Failed to write to file");

    let original_path = env::var("PATH").unwrap();
    let (valid_segment1, valid_segment2, valid_segment3) = get_valid_segments();
    let separator = get_path_separator();

    let invalid_path = format!(
        "{}{}{}{}{}{}{}",
        valid_segment1,
        separator,
        valid_segment2,
        separator,
        valid_segment3,
        separator,
        absolute_file_path.to_str().unwrap()
    );

    unsafe {
        env::set_var("PATH", invalid_path);
    }
    let matcher = SingleExecMatcher::new("true", &[], true);
    unsafe {
        env::set_var("PATH", original_path);
    }

    assert!(matcher.is_err());
}

#[test]
#[serial(path_addicted)]
fn empty_path_segment_multi_exec() {
    let original_path = env::var("PATH").unwrap();
    let (valid_segment1, valid_segment2, valid_segment3) = get_valid_segments();
    let separator = get_path_separator();

    let invalid_path = format!(
        "{}{}{}{}{}{}{}",
        valid_segment1, separator, valid_segment2, separator, valid_segment3, separator, separator
    );

    unsafe {
        env::set_var("PATH", invalid_path);
    }
    let matcher = MultiExecMatcher::new("true", &[], true);
    unsafe {
        env::set_var("PATH", original_path);
    }

    assert!(matcher.is_err());
}

#[test]
#[serial(path_addicted)]
fn relative_path_segment_multi_exec() {
    let original_path = env::var("PATH").unwrap();
    let (valid_segment1, valid_segment2, valid_segment3) = get_valid_segments();
    let separator = get_path_separator();

    let invalid_path = format!(
        "{}{}{}{}{}{}relative{}valid_segment4",
        valid_segment1, separator, valid_segment2, separator, valid_segment3, separator, separator
    );

    unsafe {
        env::set_var("PATH", invalid_path);
    }
    let matcher = MultiExecMatcher::new("true", &[], true);
    unsafe {
        env::set_var("PATH", original_path);
    }

    assert!(matcher.is_err());
}

#[test]
#[serial(path_addicted)]
fn file_path_segment_multi_exec() {
    let temp_dir = tempdir().expect("Failed to create temp file");
    let temp_dir_path = temp_dir.path();
    let absolute_file_path = temp_dir_path.join("test_file");

    File::create(&absolute_file_path)
        .expect("Failed to create file")
        .write_all(b"foo")
        .expect("Failed to write to file");

    let original_path = env::var("PATH").unwrap();
    let (valid_segment1, valid_segment2, valid_segment3) = get_valid_segments();
    let separator = get_path_separator();

    let invalid_path = format!(
        "{}{}{}{}{}{}{}",
        valid_segment1,
        separator,
        valid_segment2,
        separator,
        valid_segment3,
        separator,
        absolute_file_path.to_str().unwrap()
    );

    unsafe {
        env::set_var("PATH", invalid_path);
    }
    let matcher = MultiExecMatcher::new("true", &[], true);
    unsafe {
        env::set_var("PATH", original_path);
    }

    assert!(matcher.is_err());
}

fn get_path_separator() -> char {
    if cfg!(windows) {
        ';'
    } else {
        ':'
    }
}

fn get_valid_segments() -> (String, String, String) {
    if cfg!(windows) {
        (
            "C:\\Windows".to_string(),
            "C:\\Program Files".to_string(),
            "C:\\Program Files (x86)".to_string(),
        )
    } else {
        (
            "/usr/bin".to_string(),
            "/usr/sbin".to_string(),
            "/usr/local/bin".to_string(),
        )
    }
}
