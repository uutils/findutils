// Copyright 2021 Collabora, Ltd.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/// ! This file contains integration tests for xargs, separate from the unit
/// ! tests so that testing-commandline can be built first.
use std::io::{Seek, SeekFrom, Write};

use assert_cmd::Command;
use predicates::prelude::*;

use common::test_helpers::path_to_testing_commandline;
use pretty_assertions::assert_eq;

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
        .args(["-0n1"])
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
        .args(["-d1"])
        .write_stdin("ab1cd1ef")
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::diff("ab cd ef\n"));

    Command::cargo_bin("xargs")
        .expect("found binary")
        .args(["-d\\t", "-n1"])
        .write_stdin("a\nb\td e\tfg")
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::diff("a\nb\nd e\nfg\n"));

    Command::cargo_bin("xargs")
        .expect("found binary")
        .args(["-dabc"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("invalid"))
        .stdout(predicate::str::is_empty());
}

#[test]
fn xargs_null_conflict() {
    Command::cargo_bin("xargs")
        .expect("found binary")
        .args(["-d\t", "-0n1"])
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
        .args(["--no-run-if-empty"])
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
    for arg in ["-L2", "--max-lines=2"] {
        Command::cargo_bin("xargs")
            .expect("found binary")
            .arg(arg)
            .write_stdin("ab cd\nef\ngh i\n\njkl\n")
            .assert()
            .success()
            .stderr(predicate::str::is_empty())
            .stdout(predicate::str::diff("ab cd ef\ngh i jkl\n"));
    }
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
        // -n2 is last, so it should be given priority.
        .args(["-I=_", "-n2", "echo", "_"])
        .write_stdin("ab   cd ef\ngh i\njkl")
        .assert()
        .success()
        .stderr(predicate::str::contains("WARNING"))
        .stdout(predicate::str::diff("_ ab cd\n_ ef gh\n_ i jkl\n"));

    Command::cargo_bin("xargs")
        .expect("found binary")
        // -L2 is last, so it should be given priority.
        .args(["-n2", "-L2"])
        .write_stdin("ab cd\nef\ngh i\n\njkl\n")
        .assert()
        .success()
        .stderr(predicate::str::contains("WARNING"))
        .stdout(predicate::str::diff("ab cd ef\ngh i jkl\n"));

    Command::cargo_bin("xargs")
        .expect("found binary")
        // -L2 is last, so it should be given priority.
        .args(["-I=_", "-L2", "echo", "_"])
        .write_stdin("ab cd\nef\ngh i\n\njkl\n")
        .assert()
        .success()
        .stderr(predicate::str::contains("WARNING"))
        .stdout(predicate::str::diff("_ ab cd ef\n_ gh i jkl\n"));

    for redundant_arg in ["-L2", "-n2"] {
        Command::cargo_bin("xargs")
            .expect("found binary")
            // -I={} is last, so it should be given priority.
            .args([redundant_arg, "-I={}", "echo", "{} bar"])
            .write_stdin("ab  cd ef\ngh i\njkl")
            .assert()
            .success()
            .stderr(predicate::str::contains("WARNING"))
            .stdout(predicate::str::diff("ab  cd ef bar\ngh i bar\njkl bar\n"));
    }
}

#[test]
fn xargs_max_chars() {
    for arg in ["-s11", "--max-chars=11"] {
        Command::cargo_bin("xargs")
            .expect("found binary")
            .arg(arg)
            .write_stdin("ab cd efg")
            .assert()
            .success()
            .stderr(predicate::str::is_empty())
            .stdout(predicate::str::diff("ab cd\nefg\n"));
    }

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
    let result = Command::cargo_bin("xargs")
        .expect("found binary")
        .args([
            "-n2",
            &path_to_testing_commandline(),
            "-",
            "--print_stdin",
            "--no_print_cwd",
        ])
        .write_stdin("a b c\nd")
        .output();
    assert!(result.is_ok(), "xargs failed: {result:?}");
    let result = result.unwrap();
    assert_eq!(result.status.code(), Some(0));

    assert!(result.stderr.is_empty(), "stderr: {result:?}");

    let stdout_string = String::from_utf8(result.stdout).expect("Found invalid UTF-8");

    assert_eq!(
        stdout_string,
        "stdin=\nargs=\n--print_stdin\n--no_print_cwd\na\nb\n\
            stdin=\nargs=\n--print_stdin\n--no_print_cwd\nc\nd\n",
    );
}

#[test]
fn xargs_exec_stdin_open() {
    let mut temp_file = tempfile::NamedTempFile::new().unwrap();

    write!(temp_file, "a b c").unwrap();
    temp_file.seek(SeekFrom::Start(0)).unwrap();

    let result = Command::cargo_bin("xargs")
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
        .output();

    assert!(result.is_ok(), "xargs failed: {result:?}");
    let result = result.unwrap();
    assert_eq!(result.status.code(), Some(0));

    assert!(result.stderr.is_empty(), "stderr: {result:?}");

    let stdout_string = String::from_utf8(result.stdout).expect("Found invalid UTF-8");

    assert_eq!(
        stdout_string,
        "stdin=test\nargs=\n--print_stdin\n--no_print_cwd\na\nb\nc\n",
    );
}

#[test]
fn xargs_exec_failure() {
    let result = Command::cargo_bin("xargs")
        .expect("found binary")
        .args([
            "-n1",
            &path_to_testing_commandline(),
            "-",
            "--no_print_cwd",
            "--exit_with_failure",
        ])
        .write_stdin("a b")
        .output();

    assert!(result.is_ok(), "xargs failed: {result:?}");
    let result = result.unwrap();
    assert_eq!(result.status.code(), Some(123));

    assert!(result.stderr.is_empty(), "stderr: {result:?}");

    let stdout_string = String::from_utf8(result.stdout).expect("Found invalid UTF-8");

    assert_eq!(
        stdout_string,
        "args=\n--no_print_cwd\n--exit_with_failure\na\n\
                args=\n--no_print_cwd\n--exit_with_failure\nb\n",
    );
}

#[test]
fn xargs_exec_urgent_failure() {
    let result = Command::cargo_bin("xargs")
        .expect("found binary")
        .args([
            "-n1",
            &path_to_testing_commandline(),
            "-",
            "--no_print_cwd",
            "--exit_with_urgent_failure",
        ])
        .write_stdin("a b")
        .output();

    assert!(result.is_ok(), "xargs failed: {result:?}");
    let result = result.unwrap();
    assert_eq!(result.status.code(), Some(124));

    assert!(!result.stderr.is_empty(), "stderr: {result:?}");

    let stdout_string = String::from_utf8(result.stdout).expect("Found invalid UTF-8");

    assert_eq!(
        stdout_string,
        "args=\n--no_print_cwd\n--exit_with_urgent_failure\na\n"
    );
}

#[test]
#[cfg(unix)]
fn xargs_exec_with_signal() {
    let result = Command::cargo_bin("xargs")
        .expect("found binary")
        .args([
            "-n1",
            &path_to_testing_commandline(),
            "-",
            "--no_print_cwd",
            "--exit_with_signal",
        ])
        .write_stdin("a b")
        .output();

    assert!(result.is_ok(), "xargs failed: {result:?}");
    let result = result.unwrap();
    assert_eq!(result.status.code(), Some(125));
    assert!(!result.stderr.is_empty(), "stderr: {result:?}");

    let stdout_string = String::from_utf8(result.stdout).expect("Found invalid UTF-8");

    assert_eq!(
        stdout_string,
        "args=\n--no_print_cwd\n--exit_with_signal\na\n"
    );
}

#[test]
fn xargs_exec_not_found() {
    Command::cargo_bin("xargs")
        .expect("found binary")
        .args(["this-file-does-not-exist"])
        .assert()
        .failure()
        .code(127)
        .stderr(predicate::str::contains("Error:"))
        .stdout(predicate::str::is_empty());
}

#[test]
fn xargs_exec_verbose() {
    Command::cargo_bin("xargs")
        .expect("found binary")
        .args([
            "-n2",
            "--verbose",
            &path_to_testing_commandline(),
            "-",
            "--print_stdin",
            "--no_print_cwd",
        ])
        .write_stdin("a b c\nd")
        .assert()
        .success()
        .stderr(predicate::str::contains("testing-commandline"))
        .stdout(predicate::str::diff(
            "stdin=\nargs=\n--print_stdin\n--no_print_cwd\na\nb\n\
            stdin=\nargs=\n--print_stdin\n--no_print_cwd\nc\nd\n",
        ));
}

#[test]
fn xargs_unterminated_quote() {
    Command::cargo_bin("xargs")
        .expect("found binary")
        .args([
            "-n2",
            &path_to_testing_commandline(),
            "-",
            "--print_stdin",
            "--no_print_cwd",
        ])
        .write_stdin("a \"b c\nd")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("Error: Unterminated quote:"))
        .stdout(predicate::str::is_empty());
}

#[test]
fn xargs_zero_lines() {
    Command::cargo_bin("xargs")
        .expect("found binary")
        .args([
            "-L0",
            &path_to_testing_commandline(),
            "-",
            "--print_stdin",
            "--no_print_cwd",
        ])
        .write_stdin("a \"b c\nd")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("Value must be > 0, not: 0"))
        .stdout(predicate::str::is_empty());
}

#[test]
fn xargs_replace() {
    Command::cargo_bin("xargs")
        .expect("found binary")
        .args(["-i={}", "echo", "{} bar"])
        .write_stdin("foo")
        .assert()
        .stdout(predicate::str::contains("foo bar"));

    Command::cargo_bin("xargs")
        .expect("found binary")
        .args(["-i=_", "echo", "_ bar"])
        .write_stdin("foo")
        .assert()
        .stdout(predicate::str::contains("foo bar"));

    Command::cargo_bin("xargs")
        .expect("found binary")
        .args(["--replace=_", "echo", "_ _ bar"])
        .write_stdin("foo")
        .assert()
        .stdout(predicate::str::contains("foo foo bar"));

    Command::cargo_bin("xargs")
        .expect("found binary")
        .args(["-i=_", "echo", "_ _ bar"])
        .write_stdin("foo")
        .assert()
        .stdout(predicate::str::contains("foo foo bar"));

    Command::cargo_bin("xargs")
        .expect("found binary")
        .args(["-i", "echo", "{} {} bar"])
        .write_stdin("foo")
        .assert()
        .stdout(predicate::str::contains("foo foo bar"));

    Command::cargo_bin("xargs")
        .expect("found binary")
        .args(["-I={}", "echo", "{} bar {}"])
        .write_stdin("foo")
        .assert()
        .stdout(predicate::str::contains("foo bar foo"));

    // Combine the two options to see which one wins
    Command::cargo_bin("xargs")
        .expect("found binary")
        .args(["-I=_", "-i", "echo", "{} bar {}"])
        .write_stdin("foo")
        .assert()
        .stdout(predicate::str::contains("foo bar foo"));

    // other order
    Command::cargo_bin("xargs")
        .expect("found binary")
        .args(["-i", "-I=_", "echo", "{} bar {}"])
        .write_stdin("foo")
        .assert()
        .stdout(predicate::str::contains("{} bar {}"));

    Command::cargo_bin("xargs")
        .expect("found binary")
        .args(["-i", "-I", "_", "echo", "{} bar _"])
        .write_stdin("foo")
        .assert()
        .stdout(predicate::str::contains("{} bar foo"));

    // Expected to fail
    Command::cargo_bin("xargs")
        .expect("found binary")
        .args(["-I", "echo", "_ _ bar"])
        .write_stdin("foo")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error: Command not found"));
}

#[test]
fn xargs_replace_multiple_lines() {
    Command::cargo_bin("xargs")
        .expect("found binary")
        .args(["-I", "_", "echo", "[_]"])
        .write_stdin("ab c\nd  ef\ng")
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::diff("[ab c]\n[d  ef]\n[g]\n"));

    Command::cargo_bin("xargs")
        .expect("found binary")
        .args(["-I", "{}", "echo", "{} {} foo"])
        .write_stdin("bar\nbaz")
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::diff("bar bar foo\nbaz baz foo\n"));

    Command::cargo_bin("xargs")
        .expect("found binary")
        .args(["-I", "non-exist", "echo"])
        .write_stdin("abc\ndef\ng")
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::diff("\n\n\n"));
}
