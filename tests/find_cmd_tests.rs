// Copyright 2021 Chad Williamson <chad@dahc.us>
//
// Use of this source code is governed by an MIT-syle license that can be
// found in the LICENSE file or at https://opensource.org/licenses/MIT.

// This file contains integration tests for the find command.
//
// Note: the `serial` macro is used on tests that make assumptions about the
// working directory, since we have at least one test that needs to change it.

use assert_cmd::Command;
use predicates::prelude::*;
use serial_test::serial;
use std::env;
use std::fs::File;
use tempdir::TempDir;

#[serial(working_dir)]
#[test]
fn no_args() {
    Command::cargo_bin("find")
        .expect("found binary")
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::contains("test_data"));
}

#[serial(working_dir)]
#[test]
fn two_matchers_both_match() {
    Command::cargo_bin("find")
        .expect("found binary")
        .args(&["-type", "d", "-name", "test_data"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::contains("test_data"));
}

#[serial(working_dir)]
#[test]
fn two_matchers_one_matches() {
    Command::cargo_bin("find")
        .expect("found binary")
        .args(&["-type", "f", "-name", "test_data"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::is_empty());
}

#[test]
fn matcher_with_side_effects_at_end() {
    let temp_dir = TempDir::new("find_cmd_").expect("made temp dir");
    let temp_dir_path = temp_dir.path().to_string_lossy();
    let test_file = temp_dir.path().join("test");
    File::create(&test_file).expect("created test file");

    Command::cargo_bin("find")
        .expect("found binary")
        .args(&[&temp_dir_path, "-name", "test", "-delete"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::is_empty());

    assert!(!test_file.exists(), "test file should be deleted");
    assert!(temp_dir.path().exists(), "temp dir should NOT be deleted");
}

#[test]
fn matcher_with_side_effects_in_front() {
    let temp_dir = TempDir::new("find_cmd_").expect("made temp dir");
    let temp_dir_path = temp_dir.path().to_string_lossy();
    let test_file = temp_dir.path().join("test");
    File::create(&test_file).expect("created test file");

    Command::cargo_bin("find")
        .expect("found binary")
        .args(&[&temp_dir_path, "-delete", "-name", "test"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::is_empty());

    assert!(!test_file.exists(), "test file should be deleted");
    assert!(!temp_dir.path().exists(), "temp dir should also be deleted");
}

// This could be covered by a unit test in principle... in practice, changing
// the working dir can't be done safely in unit tests unless `--test-threads=1`
// or `serial` goes everywhere, and it doesn't seem possible to get an
// appropriate `walkdir::DirEntry` for "." without actually changing dirs
// (or risking deletion of the repo itself).
#[serial(working_dir)]
#[test]
fn delete_on_dot_dir() {
    let temp_dir = TempDir::new("find_cmd_").expect("made temp dir");
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(&temp_dir.path()).expect("working dir changed");

    Command::cargo_bin("find")
        .expect("found binary")
        .args(&[".", "-delete"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::is_empty());

    env::set_current_dir(original_dir).expect("restored original working dir");

    assert!(temp_dir.path().exists(), "temp dir should still exist");
}
