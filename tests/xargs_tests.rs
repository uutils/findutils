// Copyright 2021 Collabora, Ltd.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/// ! This file contains integration tests for xargs, separate from the unit
/// ! tests so that testing-commandline can be built first.
extern crate findutils;
extern crate tempfile;

use std::io::{Seek, SeekFrom, Write};

use assert_cmd::Command;
use predicates::prelude::*;

use common::test_helpers::*;

mod common;

#[test]
fn xargs_basics() {
    Command::cargo_bin("xargs")
        .expect("found binary")
        .write_stdin("abc\ndef g\\hi  'i  j \"k'")
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::diff("abc def ghi i  j \"k\n"));
}

#[test]
fn xargs_null() {
    Command::cargo_bin("xargs")
        .expect("found binary")
        .args(&["-0n1"])
        .write_stdin("ab c\0d\tef\0")
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::diff("ab c\nd\tef\n"));
}

#[test]
fn xargs_delim() {
    Command::cargo_bin("xargs")
        .expect("found binary")
        .args(&["-d1"])
        .write_stdin("ab1cd1ef")
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::diff("ab cd ef\n"));

    Command::cargo_bin("xargs")
        .expect("found binary")
        .args(&["-d\\t", "-n1"])
        .write_stdin("a\nb\td e\tfg")
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::diff("a\nb\nd e\nfg\n"));

    Command::cargo_bin("xargs")
        .expect("found binary")
        .args(&["-dabc"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("Invalid"))
        .stdout(predicate::str::is_empty());
}

#[test]
fn xargs_null_conflict() {
    Command::cargo_bin("xargs")
        .expect("found binary")
        .args(&["-d\t", "-0n1"])
        .write_stdin("ab c\0d\tef\0")
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::diff("ab c\nd\tef\n"));
}

#[test]
fn xargs_if_empty() {
    Command::cargo_bin("xargs")
        .expect("found binary")
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        // Should echo at least once still.
        .stdout(predicate::eq("\n"));

    Command::cargo_bin("xargs")
        .expect("found binary")
        .args(&["--no-run-if-empty"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        // Should never echo.
        .stdout(predicate::str::is_empty());
}

#[test]
fn xargs_max_args() {
    Command::cargo_bin("xargs")
        .expect("found binary")
        .args(["-n2"])
        .write_stdin("ab cd ef\ngh i")
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::diff("ab cd\nef gh\ni\n"));
}

#[test]
fn xargs_max_lines() {
    Command::cargo_bin("xargs")
        .expect("found binary")
        .args(["-L2"])
        .write_stdin("ab cd\nef\ngh i\n\njkl\n")
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::diff("ab cd ef\ngh i jkl\n"));
}

#[test]
fn xargs_max_args_lines_conflict() {
    Command::cargo_bin("xargs")
        .expect("found binary")
        // -n2 is last, so it should be given priority.
        .args(["-L2", "-n2"])
        .write_stdin("ab cd ef\ngh i")
        .assert()
        .success()
        .stderr(predicate::str::contains("WARNING"))
        .stdout(predicate::str::diff("ab cd\nef gh\ni\n"));

    Command::cargo_bin("xargs")
        .expect("found binary")
        // -L2 is last, so it should be given priority.
        .args(["-n2", "-L2"])
        .write_stdin("ab cd\nef\ngh i\n\njkl\n")
        .assert()
        .success()
        .stderr(predicate::str::contains("WARNING"))
        .stdout(predicate::str::diff("ab cd ef\ngh i jkl\n"));
}

#[test]
fn xargs_max_chars() {
    Command::cargo_bin("xargs")
        .expect("found binary")
        .args(["-s11"])
        .write_stdin("ab cd efg")
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::diff("ab cd\nefg\n"));

    // Behavior should be the same with -x, which only takes effect with -L or
    // -n.
    Command::cargo_bin("xargs")
        .expect("found binary")
        .args(["-xs11"])
        .write_stdin("ab cd efg")
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::diff("ab cd\nefg\n"));

    Command::cargo_bin("xargs")
        .expect("found binary")
        .args(["-s10"])
        .write_stdin("abcdefghijkl ab")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("Error:"))
        .stdout(predicate::str::is_empty());
}

#[test]
fn xargs_exit_on_large() {
    Command::cargo_bin("xargs")
        .expect("found binary")
        .args(["-xs11", "-n2"])
        .write_stdin("ab cd efg h i")
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::diff("ab cd\nefg h\ni\n"));

    Command::cargo_bin("xargs")
        .expect("found binary")
        .args(["-xs11", "-n2"])
        .write_stdin("abcdefg hijklmn")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("Error:"))
        .stdout(predicate::str::is_empty());
}

#[test]
fn xargs_exec() {
    Command::cargo_bin("xargs")
        .expect("found binary")
        .args([
            "-n2",
            &path_to_testing_commandline(),
            "-",
            "--print_stdin",
            "--no_print_cwd",
        ])
        .write_stdin("a b c\nd")
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::diff(
            "stdin=\nargs=\n--print_stdin\n--no_print_cwd\na\nb\n\
            stdin=\nargs=\n--print_stdin\n--no_print_cwd\nc\nd\n",
        ));
}

#[test]
fn xargs_exec_stdin_open() {
    let mut temp_file = tempfile::NamedTempFile::new().unwrap();

    write!(temp_file, "a b c").unwrap();
    temp_file.seek(SeekFrom::Start(0)).unwrap();

    Command::cargo_bin("xargs")
        .expect("found binary")
        .args([
            "-a",
            &temp_file.path().to_string_lossy(),
            &path_to_testing_commandline(),
            "-",
            "--print_stdin",
            "--no_print_cwd",
        ])
        .write_stdin("test")
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::diff(
            "stdin=test\nargs=\n--print_stdin\n--no_print_cwd\na\nb\nc\n",
        ));
}

#[test]
fn xargs_exec_failure() {
    Command::cargo_bin("xargs")
        .expect("found binary")
        .args(&[
            "-n1",
            &path_to_testing_commandline(),
            "-",
            "--no_print_cwd",
            "--exit_with_failure",
        ])
        .write_stdin("a b")
        .assert()
        .failure()
        .code(123)
        .stderr(predicate::str::is_empty())
        .stdout(
            "args=\n--no_print_cwd\n--exit_with_failure\na\n\
                args=\n--no_print_cwd\n--exit_with_failure\nb\n",
        );
}

#[test]
fn xargs_exec_urgent_failure() {
    Command::cargo_bin("xargs")
        .expect("found binary")
        .args(&[
            "-n1",
            &path_to_testing_commandline(),
            "-",
            "--no_print_cwd",
            "--exit_with_urgent_failure",
        ])
        .write_stdin("a b")
        .assert()
        .failure()
        .code(124)
        .stderr(predicate::str::contains("Error:"))
        .stdout("args=\n--no_print_cwd\n--exit_with_urgent_failure\na\n");
}

#[test]
#[cfg(unix)]
fn xargs_exec_with_signal() {
    Command::cargo_bin("xargs")
        .expect("found binary")
        .args(&[
            "-n1",
            &path_to_testing_commandline(),
            "-",
            "--no_print_cwd",
            "--exit_with_signal",
        ])
        .write_stdin("a b")
        .assert()
        .failure()
        .code(125)
        .stderr(predicate::str::contains("Error:"))
        .stdout("args=\n--no_print_cwd\n--exit_with_signal\na\n");
}

#[test]
fn xargs_exec_not_found() {
    Command::cargo_bin("xargs")
        .expect("found binary")
        .args(&["this-file-does-not-exist"])
        .assert()
        .failure()
        .code(127)
        .stderr(predicate::str::contains("Error:"))
        .stdout(predicate::str::is_empty());
}
