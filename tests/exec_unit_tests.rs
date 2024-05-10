// Copyright 2017 Google Inc.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/// ! This file contains what would be normally be unit tests for `find::matchers::exec`.
/// ! But as the tests require running an external executable, they need to be run
/// ! as integration tests so we can ensure that our testing-commandline binary
/// ! has been built.
use std::env;
use std::fs::File;
use std::io::Read;
use tempfile::Builder;

use common::test_helpers::{
    fix_up_slashes, get_dir_entry_for, path_to_testing_commandline, FakeDependencies,
};
use findutils::find::matchers::exec::SingleExecMatcher;
use findutils::find::matchers::Matcher;

mod common;

#[test]
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
            "cwd={}/test_data/simple\nargs=\nabc\n./abbbc\nxyz\n",
            env::current_dir().unwrap().to_string_lossy()
        ))
    );
}

#[test]
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
        false
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
        false,
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
        false,
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
        false,
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
