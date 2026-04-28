// Copyright 2021 Collabora, Ltd.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

// Integration tests for the xargs command using the uutests framework.
// These are integration tests (rather than unit tests) so that the
// testing-commandline binary is guaranteed to be built first.
use uutests::util::TestScenario;

use common::test_helpers::path_to_testing_commandline;

mod common;

fn ucmd() -> uutests::util::UCommand {
    TestScenario::new("xargs").cmd(env!("CARGO_BIN_EXE_xargs"))
}

#[test]
fn xargs_basics() {
    ucmd()
        .pipe_in("abc\ndef g\\hi  'i  j \"k'")
        .succeeds()
        .stdout_only("abc def ghi i  j \"k\n");
}

#[test]
fn xargs_null() {
    ucmd()
        .args(&["-0n1"])
        .pipe_in("ab c\0d\tef\0")
        .succeeds()
        .stdout_only("ab c\nd\tef\n");
}

#[test]
fn xargs_delim() {
    ucmd()
        .args(&["-d1"])
        .pipe_in("ab1cd1ef")
        .succeeds()
        .stdout_only("ab cd ef\n");

    ucmd()
        .args(&["-d\\t", "-n1"])
        .pipe_in("a\nb\td e\tfg")
        .succeeds()
        .stdout_only("a\nb\nd e\nfg\n");

    ucmd()
        .args(&["-dabc"])
        .fails_with_code(1)
        .stderr_contains("invalid")
        .no_stdout();
}

#[test]
fn xargs_null_conflict() {
    ucmd()
        .args(&["-d\t", "-0n1"])
        .pipe_in("ab c\0d\tef\0")
        .succeeds()
        .stdout_only("ab c\nd\tef\n");
}

#[test]
fn xargs_if_empty() {
    // Should echo at least once still.
    ucmd().succeeds().no_stderr().stdout_only("\n");

    // Should never echo.
    ucmd().args(&["--no-run-if-empty"]).succeeds().no_output();
}

#[test]
fn xargs_max_args() {
    ucmd()
        .args(&["-n2"])
        .pipe_in("ab cd ef\ngh i")
        .succeeds()
        .stdout_only("ab cd\nef gh\ni\n");
}

#[test]
fn xargs_max_lines() {
    for arg in ["-L2", "--max-lines=2", "-l2"] {
        ucmd()
            .arg(arg)
            .pipe_in("ab cd\nef\ngh i\n\njkl\n")
            .succeeds()
            .stdout_only("ab cd ef\ngh i jkl\n");
    }
    ucmd()
        .arg("-l")
        .pipe_in("ab cd\nef\ngh i\n\njkl\n")
        .succeeds()
        .stdout_only("ab cd\nef\ngh i\njkl\n");
}

#[test]
fn xargs_max_args_lines_conflict() {
    // -n2 is last, so it should be given priority.
    ucmd()
        .args(&["-L2", "-n2"])
        .pipe_in("ab cd ef\ngh i")
        .succeeds()
        .stderr_contains("WARNING")
        .stdout_is("ab cd\nef gh\ni\n");

    // -n2 is last, so it should be given priority.
    ucmd()
        .args(&["-I=_", "-n2", "echo", "_"])
        .pipe_in("ab   cd ef\ngh i\njkl")
        .succeeds()
        .stderr_contains("WARNING")
        .stdout_is("_ ab cd\n_ ef gh\n_ i jkl\n");

    // -L2 is last, so it should be given priority.
    ucmd()
        .args(&["-n2", "-L2"])
        .pipe_in("ab cd\nef\ngh i\n\njkl\n")
        .succeeds()
        .stderr_contains("WARNING")
        .stdout_is("ab cd ef\ngh i jkl\n");

    // -L2 is last, so it should be given priority.
    ucmd()
        .args(&["-I=_", "-L2", "echo", "_"])
        .pipe_in("ab cd\nef\ngh i\n\njkl\n")
        .succeeds()
        .stderr_contains("WARNING")
        .stdout_is("_ ab cd ef\n_ gh i jkl\n");

    for redundant_arg in ["-L2", "-n2"] {
        // -I={} is last, so it should be given priority.
        ucmd()
            .args(&[redundant_arg, "-I={}", "echo", "{} bar"])
            .pipe_in("ab  cd ef\ngh i\njkl")
            .succeeds()
            .stderr_contains("WARNING")
            .stdout_is("ab  cd ef bar\ngh i bar\njkl bar\n");
    }
}

#[test]
fn xargs_max_chars() {
    for arg in ["-s11", "--max-chars=11"] {
        ucmd()
            .arg(arg)
            .pipe_in("ab cd efg")
            .succeeds()
            .stdout_only("ab cd\nefg\n");
    }

    // Behavior should be the same with -x, which only takes effect with -L or
    // -n.
    ucmd()
        .args(&["-xs11"])
        .pipe_in("ab cd efg")
        .succeeds()
        .stdout_only("ab cd\nefg\n");

    ucmd()
        .args(&["-s10"])
        .pipe_in("abcdefghijkl ab")
        .fails_with_code(1)
        .stderr_contains("Error:")
        .no_stdout();
}

#[test]
fn xargs_exit_on_large() {
    ucmd()
        .args(&["-xs11", "-n2"])
        .pipe_in("ab cd efg h i")
        .succeeds()
        .stdout_only("ab cd\nefg h\ni\n");

    ucmd()
        .args(&["-xs11", "-n2"])
        .pipe_in("abcdefg hijklmn")
        .fails_with_code(1)
        .stderr_contains("Error:")
        .no_stdout();
}

#[test]
fn xargs_exec() {
    let result = ucmd()
        .args(&[
            "-n2",
            &path_to_testing_commandline(),
            "-",
            "--print_stdin",
            "--no_print_cwd",
        ])
        .pipe_in("a b c\nd")
        .succeeds();
    result.no_stderr();
    assert_eq!(
        result.stdout_str(),
        "stdin=\nargs=\n--print_stdin\n--no_print_cwd\na\nb\n\
            stdin=\nargs=\n--print_stdin\n--no_print_cwd\nc\nd\n",
    );
}

#[test]
fn xargs_exec_stdin_open() {
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(temp_file.path(), b"a b c").unwrap();

    let result = ucmd()
        .args(&[
            "-a",
            &temp_file.path().to_string_lossy(),
            &path_to_testing_commandline(),
            "-",
            "--print_stdin",
            "--no_print_cwd",
        ])
        .pipe_in("test")
        .succeeds();
    result.no_stderr();
    assert_eq!(
        result.stdout_str(),
        "stdin=test\nargs=\n--print_stdin\n--no_print_cwd\na\nb\nc\n",
    );
}

#[test]
fn xargs_exec_failure() {
    let result = ucmd()
        .args(&[
            "-n1",
            &path_to_testing_commandline(),
            "-",
            "--no_print_cwd",
            "--exit_with_failure",
        ])
        .pipe_in("a b")
        .run();
    result.code_is(123);
    result.no_stderr();
    assert_eq!(
        result.stdout_str(),
        "args=\n--no_print_cwd\n--exit_with_failure\na\n\
                args=\n--no_print_cwd\n--exit_with_failure\nb\n",
    );
}

#[test]
fn xargs_exec_urgent_failure() {
    let result = ucmd()
        .args(&[
            "-n1",
            &path_to_testing_commandline(),
            "-",
            "--no_print_cwd",
            "--exit_with_urgent_failure",
        ])
        .pipe_in("a b")
        .run();
    result.code_is(124);
    assert!(
        !result.stderr_str().is_empty(),
        "stderr should not be empty"
    );
    assert_eq!(
        result.stdout_str(),
        "args=\n--no_print_cwd\n--exit_with_urgent_failure\na\n"
    );
}

#[test]
#[cfg(unix)]
fn xargs_exec_with_signal() {
    let result = ucmd()
        .args(&[
            "-n1",
            &path_to_testing_commandline(),
            "-",
            "--no_print_cwd",
            "--exit_with_signal",
        ])
        .pipe_in("a b")
        .run();
    result.code_is(125);
    assert!(
        !result.stderr_str().is_empty(),
        "stderr should not be empty"
    );
    assert_eq!(
        result.stdout_str(),
        "args=\n--no_print_cwd\n--exit_with_signal\na\n"
    );
}

#[test]
fn xargs_exec_not_found() {
    ucmd()
        .args(&["this-file-does-not-exist"])
        .fails_with_code(127)
        .stderr_contains("Error:")
        .no_stdout();
}

#[test]
fn xargs_exec_verbose() {
    ucmd()
        .args(&[
            "-n2",
            "--verbose",
            &path_to_testing_commandline(),
            "-",
            "--print_stdin",
            "--no_print_cwd",
        ])
        .pipe_in("a b c\nd")
        .succeeds()
        .stderr_contains("testing-commandline")
        .stdout_is(
            "stdin=\nargs=\n--print_stdin\n--no_print_cwd\na\nb\n\
            stdin=\nargs=\n--print_stdin\n--no_print_cwd\nc\nd\n",
        );
}

#[test]
fn xargs_unterminated_quote() {
    ucmd()
        .args(&[
            "-n2",
            &path_to_testing_commandline(),
            "-",
            "--print_stdin",
            "--no_print_cwd",
        ])
        .pipe_in("a \"b c\nd")
        .fails_with_code(1)
        .stderr_contains("Error: Unterminated quote:")
        .no_stdout();
}

#[test]
fn xargs_zero_lines() {
    ucmd()
        .args(&[
            "-L0",
            &path_to_testing_commandline(),
            "-",
            "--print_stdin",
            "--no_print_cwd",
        ])
        // Empty stdin: -L0 is rejected before reading input, so writing
        // actual content races with xargs exiting and causes a broken pipe.
        .pipe_in("")
        .fails_with_code(1)
        .stderr_contains("Value must be > 0, not: 0")
        .no_stdout();
}

#[test]
fn xargs_replace() {
    ucmd()
        .args(&["-i={}", "echo", "{} bar"])
        .pipe_in("foo")
        .succeeds()
        .stdout_contains("foo bar");

    ucmd()
        .args(&["-i=_", "echo", "_ bar"])
        .pipe_in("foo")
        .succeeds()
        .stdout_contains("foo bar");

    ucmd()
        .args(&["--replace=_", "echo", "_ _ bar"])
        .pipe_in("foo")
        .succeeds()
        .stdout_contains("foo foo bar");

    ucmd()
        .args(&["-i=_", "echo", "_ _ bar"])
        .pipe_in("foo")
        .succeeds()
        .stdout_contains("foo foo bar");

    ucmd()
        .args(&["-i", "echo", "{} {} bar"])
        .pipe_in("foo")
        .succeeds()
        .stdout_contains("foo foo bar");

    ucmd()
        .args(&["-I={}", "echo", "{} bar {}"])
        .pipe_in("foo")
        .succeeds()
        .stdout_contains("foo bar foo");

    // Combine the two options to see which one wins
    ucmd()
        .args(&["-I=_", "-i", "echo", "{} bar {}"])
        .pipe_in("foo")
        .succeeds()
        .stdout_contains("foo bar foo");

    // other order
    ucmd()
        .args(&["-i", "-I=_", "echo", "{} bar {}"])
        .pipe_in("foo")
        .succeeds()
        .stdout_contains("{} bar {}");

    ucmd()
        .args(&["-i", "-I", "_", "echo", "{} bar _"])
        .pipe_in("foo")
        .succeeds()
        .stdout_contains("{} bar foo");

    ucmd()
        .args(&["-I", "-_", "echo", "-_ bar"])
        .pipe_in("foo")
        .succeeds()
        .stdout_contains("foo bar");

    // Expected to fail
    ucmd()
        .args(&["-I", "echo", "_ _ bar"])
        .pipe_in("foo")
        .fails()
        .stderr_contains("Error: Command not found");
}

#[test]
fn xargs_replace_multiple_lines() {
    ucmd()
        .args(&["-I", "_", "echo", "[_]"])
        .pipe_in("ab c\nd  ef\ng")
        .succeeds()
        .stdout_only("[ab c]\n[d  ef]\n[g]\n");

    ucmd()
        .args(&["-I", "{}", "echo", "{} {} foo"])
        .pipe_in("bar\nbaz")
        .succeeds()
        .stdout_only("bar bar foo\nbaz baz foo\n");

    ucmd()
        .args(&["-I", "non-exist", "echo"])
        .pipe_in("abc\ndef\ng")
        .succeeds()
        .stdout_only("\n\n\n");
}

#[test]
fn xargs_help() {
    for option_style in ["-h", "--help"] {
        ucmd()
            .args(&[option_style])
            .succeeds()
            .no_stderr()
            .stdout_contains("--help");
    }
}

// Do not regress to:
//
// ❯ xargs --version
// Error: xargs 0.7.0
//
// Same for help above.
#[test]
fn xargs_version() {
    for option_style in ["-V", "--version"] {
        let output = ucmd().args(&[option_style]).succeeds();
        let result = output.no_stderr();
        assert!(
            result.stdout_str().starts_with("xargs "),
            "expected stdout to start with 'xargs ', got: {:?}",
            result.stdout_str()
        );
    }
}

#[test]
fn xargs_eof() {
    for option_style in [vec!["-ecd"], vec!["-E", "cd"], vec!["--eof", "cd"]] {
        ucmd()
            .args(option_style.as_slice())
            .pipe_in("ab cd ef")
            .succeeds()
            .stdout_only("ab\n");
    }

    ucmd()
        .args(&["-E", "-end"])
        .pipe_in("ab -end ef")
        .succeeds()
        .stdout_only("ab\n");
}

#[test]
fn xargs_eof_with_delimiter() {
    ucmd()
        .args(&["-0", "-Ecd"])
        .pipe_in("ab\0cd\0ef")
        .succeeds()
        .stdout_only("ab cd ef\n");
}
