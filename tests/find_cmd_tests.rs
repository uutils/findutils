// Copyright 2021 Chad Williamson <chad@dahc.us>
//
// Use of this source code is governed by an MIT-style license that can be
// found in the LICENSE file or at https://opensource.org/licenses/MIT.

// This file contains integration tests for the find command.
//
// Note: the `serial` macro is used on tests that make assumptions about the
// working directory, since we have at least one test that needs to change it.

use assert_cmd::Command;
use predicates::prelude::*;
use regex::Regex;
use serial_test::serial;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::{env, io::ErrorKind};
use tempfile::Builder;

#[cfg(unix)]
use std::os::unix::fs::symlink;

#[cfg(windows)]
use std::os::windows::fs::{symlink_dir, symlink_file};

use common::test_helpers::fix_up_slashes;

mod common;

// Variants of fix_up_slashes that properly escape the forward slashes for being
// in a regex.
#[cfg(windows)]
fn fix_up_regex_slashes(re: &str) -> String {
    re.replace("/", "\\\\")
}

#[cfg(not(windows))]
fn fix_up_regex_slashes(re: &str) -> String {
    re.to_owned()
}

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
        .args(["-type", "d", "-name", "test_data"])
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
        .args(["-type", "f", "-name", "test_data"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::is_empty());
}

#[serial(working_dir)]
#[test]
fn files0_empty_file() {
    Command::cargo_bin("find")
        .expect("found binary")
        .args(["-files0-from", "./test_data/simple/abbbc"])
        .assert()
        .failure()
        .stderr(predicate::str::diff(
            "Error: Empty starting point detected in -files0-from input OR File is empty\n",
        ))
        .stdout(predicate::str::is_empty());
}

#[serial(working_dir)]
#[test]
fn files0_empty_pipe() {
    Command::cargo_bin("find")
        .expect("found binary")
        .args(["-files0-from", "-"])
        .write_stdin(b"")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Error: Empty starting point detected in -files0-from input",
        ))
        .stdout(predicate::str::is_empty());
}

#[serial(working_dir)]
#[test]
fn files0_pipe_basic() {
    Command::cargo_bin("find")
        .expect("found binary")
        .write_stdin(b"./test_data/simple\0./test_data/links")
        .args(["-files0-from", "-"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::contains("./test_data/"));
}

#[serial(working_dir)]
#[test]
fn files0_pipe_double_nul() {
    Command::cargo_bin("find")
        .expect("found binary")
        .write_stdin(b"./test_data/simple\0\0./test_data/links")
        .args(["-files0-from", "-"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Error: Empty starting point detected in -files0-from input",
        ))
        .stdout(predicate::str::is_empty());
}

#[test]
fn matcher_with_side_effects_at_end() {
    let temp_dir = Builder::new().prefix("find_cmd_").tempdir().unwrap();

    let temp_dir_path = temp_dir.path().to_string_lossy();
    let test_file = temp_dir.path().join("test");
    File::create(&test_file).expect("created test file");

    Command::cargo_bin("find")
        .expect("found binary")
        .args([&temp_dir_path, "-name", "test", "-delete"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::is_empty());

    assert!(!test_file.exists(), "test file should be deleted");
    assert!(temp_dir.path().exists(), "temp dir should NOT be deleted");
}

#[test]
fn matcher_with_side_effects_in_front() {
    let temp_dir = Builder::new().prefix("find_cmd_").tempdir().unwrap();

    let temp_dir_path = temp_dir.path().to_string_lossy();
    let test_file = temp_dir.path().join("test");
    File::create(&test_file).expect("created test file");

    Command::cargo_bin("find")
        .expect("found binary")
        .args([&temp_dir_path, "-delete", "-name", "test"])
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
    let temp_dir = Builder::new().prefix("example").tempdir().unwrap();
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).expect("working dir changed");

    // "." should be matched (confirmed by the print), but not deleted.
    Command::cargo_bin("find")
        .expect("found binary")
        .args([".", "-delete", "-print"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::diff(".\n"));

    env::set_current_dir(original_dir).expect("restored original working dir");

    assert!(temp_dir.path().exists(), "temp dir should still exist");
}

#[test]
fn regex_types() {
    let temp_dir = Builder::new().prefix("find_cmd_").tempdir().unwrap();

    let temp_dir_path = temp_dir.path().to_string_lossy();
    let test_file = temp_dir.path().join("teeest");
    File::create(test_file).expect("created test file");

    Command::cargo_bin("find")
        .expect("found binary")
        .args([&temp_dir_path, "-regex", &fix_up_regex_slashes(".*/tE+st")])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::is_empty());

    Command::cargo_bin("find")
        .expect("found binary")
        .args([&temp_dir_path, "-iregex", &fix_up_regex_slashes(".*/tE+st")])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::contains("teeest"));

    Command::cargo_bin("find")
        .expect("found binary")
        .args([
            &temp_dir_path,
            "-regextype",
            "posix-basic",
            "-regex",
            &fix_up_regex_slashes(r".*/te\{1,3\}st"),
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::contains("teeest"));

    Command::cargo_bin("find")
        .expect("found binary")
        .args([
            &temp_dir_path,
            "-regextype",
            "posix-extended",
            "-regex",
            &fix_up_regex_slashes(".*/te{1,3}st"),
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::contains("teeest"));

    Command::cargo_bin("find")
        .expect("found binary")
        .args([
            &temp_dir_path,
            "-regextype",
            "ed",
            "-regex",
            &fix_up_regex_slashes(r".*/te\{1,3\}st"),
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::contains("teeest"));

    Command::cargo_bin("find")
        .expect("found binary")
        .args([
            &temp_dir_path,
            "-regextype",
            "sed",
            "-regex",
            &fix_up_regex_slashes(r".*/te\{1,3\}st"),
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::contains("teeest"));
}

#[test]
fn empty_files() {
    let temp_dir = Builder::new().prefix("find_cmd_").tempdir().unwrap();
    let temp_dir_path = temp_dir.path().to_string_lossy();

    Command::cargo_bin("find")
        .expect("found binary")
        .args([&temp_dir_path, "-empty"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(fix_up_slashes(&format!("{temp_dir_path}\n")));

    let test_file_path = temp_dir.path().join("test");
    let mut test_file = File::create(&test_file_path).unwrap();

    Command::cargo_bin("find")
        .expect("found binary")
        .args([&temp_dir_path, "-empty"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(fix_up_slashes(&format!(
            "{}\n",
            test_file_path.to_string_lossy()
        )));

    let subdir_path = temp_dir.path().join("subdir");
    std::fs::create_dir(&subdir_path).unwrap();

    Command::cargo_bin("find")
        .expect("found binary")
        .args([&temp_dir_path, "-empty", "-sorted"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(fix_up_slashes(&format!(
            "{}\n{}\n",
            subdir_path.to_string_lossy(),
            test_file_path.to_string_lossy()
        )));

    write!(test_file, "x").unwrap();
    test_file.sync_all().unwrap();

    Command::cargo_bin("find")
        .expect("found binary")
        .args([&temp_dir_path, "-empty", "-sorted"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(fix_up_slashes(&format!(
            "{}\n",
            subdir_path.to_string_lossy(),
        )));
}

#[serial(working_dir)]
#[test]
fn find_printf() {
    #[cfg(unix)]
    {
        if let Err(e) = symlink("abbbc", "test_data/links/link-f") {
            assert!(
                e.kind() == ErrorKind::AlreadyExists,
                "Failed to create sym link: {e:?}"
            );
        }
        if let Err(e) = symlink("subdir", "test_data/links/link-d") {
            assert!(
                e.kind() == ErrorKind::AlreadyExists,
                "Failed to create sym link: {e:?}"
            );
        }
        if let Err(e) = symlink("missing", "test_data/links/link-missing") {
            assert!(
                e.kind() == ErrorKind::AlreadyExists,
                "Failed to create sym link: {e:?}"
            );
        }
        if let Err(e) = symlink("abbbc/x", "test_data/links/link-notdir") {
            assert!(
                e.kind() == ErrorKind::AlreadyExists,
                "Failed to create sym link: {e:?}"
            );
        }
        if let Err(e) = symlink("link-loop", "test_data/links/link-loop") {
            assert!(
                e.kind() == ErrorKind::AlreadyExists,
                "Failed to create sym link: {e:?}"
            );
        }
    }
    #[cfg(windows)]
    {
        if let Err(e) = symlink_file("abbbc", "test_data/links/link-f") {
            assert!(
                e.kind() == ErrorKind::AlreadyExists,
                "Failed to create sym link: {:?}",
                e
            );
        }
        if let Err(e) = symlink_dir("subdir", "test_data/links/link-d") {
            assert!(
                e.kind() == ErrorKind::AlreadyExists,
                "Failed to create sym link: {:?}",
                e
            );
        }
        if let Err(e) = symlink_file("missing", "test_data/links/link-missing") {
            assert!(
                e.kind() == ErrorKind::AlreadyExists,
                "Failed to create sym link: {:?}",
                e
            );
        }
        if let Err(e) = symlink_file("abbbc/x", "test_data/links/link-notdir") {
            assert!(
                e.kind() == ErrorKind::AlreadyExists,
                "Failed to create sym link: {:?}",
                e
            );
        }
    }

    Command::cargo_bin("find")
        .expect("found binary")
        .args([
            &fix_up_slashes("./test_data/simple"),
            "-sorted",
            "-printf",
            "%f %d %h %H %p %P %y\n",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::diff(fix_up_slashes(
            "simple 0 ./test_data ./test_data/simple \
            ./test_data/simple  d\n\
            abbbc 1 ./test_data/simple ./test_data/simple \
            ./test_data/simple/abbbc abbbc f\n\
            subdir 1 ./test_data/simple ./test_data/simple \
            ./test_data/simple/subdir subdir d\n\
            ABBBC 2 ./test_data/simple/subdir ./test_data/simple \
            ./test_data/simple/subdir/ABBBC subdir/ABBBC f\n",
        )));

    fs::create_dir_all("a").expect("Failed to create directory 'a'");
    let output = Command::cargo_bin("find")
        .expect("found binary")
        .args(["a", "-printf", "%A+"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).expect("Invalid UTF-8 in output");

    println!("Actual output: '{}'", output_str.trim());

    let re = Regex::new(r"^\d{4}-\d{2}-\d{2}\+\d{2}:\d{2}:\d{2}\.\d{9}0$")
        .expect("Failed to compile regex");

    assert!(
        re.is_match(output_str.trim()),
        "Output did not match expected timestamp format"
    );

    Command::cargo_bin("find")
        .expect("found binary")
        .args([
            &fix_up_slashes("./test_data/links"),
            "-sorted",
            "-type",
            "l",
            "-printf",
            "%f %l %y %Y\n",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::diff(
            [
                "link-d subdir l d\n",
                "link-f abbbc l f\n",
                #[cfg(unix)]
                "link-loop link-loop l L\n",
                "link-missing missing l N\n",
                // We can't detect ENOTDIR on non-unix platforms yet.
                #[cfg(not(unix))]
                "link-notdir abbbc/x l ?\n",
                #[cfg(unix)]
                "link-notdir abbbc/x l N\n",
            ]
            .join(""),
        ));
}

#[cfg(unix)]
#[serial(working_dir)]
#[test]
fn find_perm() {
    Command::cargo_bin("find")
        .expect("found binary")
        .args(["-perm", "+rwx"])
        .assert()
        .success();

    Command::cargo_bin("find")
        .expect("found binary")
        .args(["-perm", "u+rwX"])
        .assert()
        .success();

    Command::cargo_bin("find")
        .expect("found binary")
        .args(["-perm", "u=g"])
        .assert()
        .success();
}

#[cfg(unix)]
#[serial(working_dir)]
#[test]
fn find_inum() {
    use std::fs::metadata;
    use std::os::unix::fs::MetadataExt;

    let inum = metadata("test_data/simple/abbbc")
        .expect("metadata for abbbc")
        .ino()
        .to_string();

    Command::cargo_bin("find")
        .expect("found binary")
        .args(["test_data", "-inum", &inum])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::contains("abbbc"));
}

#[cfg(not(unix))]
#[test]
fn find_inum() {
    Command::cargo_bin("find")
        .expect("found binary")
        .args(["test_data", "-inum", "1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not available on this platform"))
        .stdout(predicate::str::is_empty());
}

#[cfg(unix)]
#[serial(working_dir)]
#[test]
fn find_links() {
    Command::cargo_bin("find")
        .expect("found binary")
        .args(["test_data", "-links", "1"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::contains("abbbc"));
}

#[cfg(not(unix))]
#[test]
fn find_links() {
    Command::cargo_bin("find")
        .expect("found binary")
        .args(["test_data", "-links", "1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not available on this platform"))
        .stdout(predicate::str::is_empty());
}

#[serial(working_dir)]
#[test]
fn find_mount_xdev() {
    // Make sure that -mount/-xdev doesn't prune unexpectedly.
    // TODO: Test with a mount point in the search.

    Command::cargo_bin("find")
        .expect("found binary")
        .args(["test_data", "-mount"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::contains("abbbc"));

    Command::cargo_bin("find")
        .expect("found binary")
        .args(["test_data", "-xdev"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::contains("abbbc"));
}

#[serial(working_dir)]
#[test]
fn find_accessible() {
    Command::cargo_bin("find")
        .expect("found binary")
        .args(["test_data", "-readable"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::contains("abbbc"));

    Command::cargo_bin("find")
        .expect("found binary")
        .args(["test_data", "-writable"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::contains("abbbc"));

    #[cfg(unix)]
    Command::cargo_bin("find")
        .expect("found binary")
        .args(["test_data", "-executable"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::contains("abbbc").not());
}

#[serial(working_dir)]
#[test]
fn find_time() {
    let args = ["1", "+1", "-1"];
    let exception_args = ["1%2", "1%2%3", "1a2", "1%2a", "abc", "-", "+", "%"];

    let tests = [
        "-atime",
        #[cfg(unix)]
        "-ctime",
        "-mtime",
    ];
    tests.iter().for_each(|flag| {
        args.iter().for_each(|arg| {
            Command::cargo_bin("find")
                .expect("found binary")
                .args(["./test_data/simple", flag, arg])
                .assert()
                .success()
                .stderr(predicate::str::is_empty());
        });

        exception_args.iter().for_each(|arg| {
            Command::cargo_bin("find")
                .expect("found binary")
                .args([".", flag, arg])
                .assert()
                .failure()
                .stdout(predicate::str::is_empty());
        });
    });
}

#[test]
fn expression_empty_parentheses() {
    Command::cargo_bin("find")
        .expect("found binary")
        .args(["-true", "(", ")"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "empty parentheses are not allowed",
        ))
        .stdout(predicate::str::is_empty());
}

#[test]
#[cfg(unix)]
#[serial(working_dir)]
fn find_with_user_predicate() {
    // Considering the different test environments,
    // the test code can only use a specific default user to perform the test,
    // such as the root user on Linux.
    Command::cargo_bin("find")
        .expect("found binary")
        .args(["test_data", "-user", "root"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    Command::cargo_bin("find")
        .expect("found binary")
        .args(["test_data", "-user", ""])
        .assert()
        .failure()
        .stderr(predicate::str::contains("empty"))
        .stdout(predicate::str::is_empty());

    Command::cargo_bin("find")
        .expect("found binary")
        .args(["test_data", "-user", " "])
        .assert()
        .failure()
        .stderr(predicate::str::contains("is not the name of a known user"))
        .stdout(predicate::str::is_empty());
}

#[test]
#[serial(working_dir)]
fn find_with_nouser_predicate() {
    Command::cargo_bin("find")
        .expect("found binary")
        .args(["test_data", "-nouser"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
#[cfg(unix)]
#[serial(working_dir)]
fn find_with_uid_predicate() {
    use std::os::unix::fs::MetadataExt;
    use std::path::Path;

    let path = Path::new("./test_data");
    let uid = path.metadata().unwrap().uid();

    Command::cargo_bin("find")
        .expect("found binary")
        .args(["test_data", "-uid", &uid.to_string()])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
#[serial(working_dir)]
fn find_with_group_predicate() {
    // Considering the different test environments,
    // the test code can only use a specific default user group for the test,
    // such as the root user group on Linux.
    #[cfg(target_os = "linux")]
    Command::cargo_bin("find")
        .expect("found binary")
        .args(["test_data", "-group", "root"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    #[cfg(target_os = "macos")]
    Command::cargo_bin("find")
        .expect("found binary")
        .args(["test_data", "-group", "staff"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    Command::cargo_bin("find")
        .expect("found binary")
        .args(["test_data", "-group", ""])
        .assert()
        .failure()
        .stderr(predicate::str::contains("empty"))
        .stdout(predicate::str::is_empty());

    Command::cargo_bin("find")
        .expect("found binary")
        .args(["test_data", "-group", " "])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "is not the name of an existing group",
        ))
        .stdout(predicate::str::is_empty());
}

#[test]
#[serial(working_dir)]
fn find_with_nogroup_predicate() {
    Command::cargo_bin("find")
        .expect("found binary")
        .args(["test_data", "-nogroup"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
#[cfg(unix)]
#[serial(working_dir)]
fn find_with_gid_predicate() {
    use std::os::unix::fs::MetadataExt;
    use std::path::Path;

    let path = Path::new("./test_data");
    let gid = path.metadata().unwrap().gid();

    Command::cargo_bin("find")
        .expect("found binary")
        .args(["test_data", "-gid", &gid.to_string()])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
#[serial(working_dir)]
fn find_newer_xy() {
    let options = [
        "a",
        #[cfg(not(target_os = "linux"))]
        "B",
        #[cfg(unix)]
        "c",
        "m",
    ];

    for x in options {
        for y in options {
            let arg = &format!("-newer{x}{y}");
            Command::cargo_bin("find")
                .expect("found binary")
                .args([
                    "./test_data/simple/subdir",
                    arg,
                    "./test_data/simple/subdir/ABBBC",
                ])
                .assert()
                .success()
                .stderr(predicate::str::is_empty());
        }
    }

    let args = [
        "-newerat",
        #[cfg(not(target_os = "linux"))]
        "-newerBt",
        #[cfg(unix)]
        "-newerct",
        "-newermt",
    ];
    let times = ["jan 01, 2000", "jan 01, 2000 00:00:00"];

    for arg in args {
        for time in times {
            Command::cargo_bin("find")
                .expect("found binary")
                .args(["./test_data/simple/subdir", arg, time])
                .assert()
                .success()
                .stderr(predicate::str::is_empty());
        }
    }
}

#[test]
#[serial(working_dir)]
fn find_age_range() {
    let args = ["-amin", "-cmin", "-mmin"];
    let times = ["-60", "-120", "-240", "+60", "+120", "+240"];
    let time_strings = [
        "\"-60\"", "\"-120\"", "\"-240\"", "\"-60\"", "\"-120\"", "\"-240\"",
    ];

    for arg in args {
        for time in times {
            Command::cargo_bin("find")
                .expect("the time should match")
                .args(["test_data/simple", arg, time])
                .assert()
                .success()
                .code(0);
        }
    }

    for arg in args {
        for time_string in time_strings {
            Command::cargo_bin("find")
                .expect("the time should not match")
                .args(["test_data/simple", arg, time_string])
                .assert()
                .failure()
                .code(1)
                .stderr(predicate::str::contains(
                    "Error: Expected a decimal integer (with optional + or - prefix) argument to",
                ))
                .stdout(predicate::str::is_empty());
        }
    }
}

#[test]
#[cfg(unix)]
#[serial(working_dir)]
fn find_fs() {
    use findutils::find::matchers::fs::get_file_system_type;
    use std::cell::RefCell;
    use std::path::Path;

    let path = Path::new("./test_data/simple/subdir");
    let empty_cache = RefCell::new(None);
    let target_fs_type = get_file_system_type(path, &empty_cache).unwrap();

    // match fs type
    Command::cargo_bin("find")
        .expect("found binary")
        .args(["./test_data/simple/subdir", "-fstype", &target_fs_type])
        .assert()
        .success()
        .stdout(predicate::str::contains("./test_data/simple/subdir"))
        .stderr(predicate::str::is_empty());

    // not match fs type
    Command::cargo_bin("find")
        .expect("found binary")
        .args([
            "./test_data/simple/subdir",
            "-fstype",
            format!("{} foo", target_fs_type).as_str(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());

    // not contain fstype text.
    Command::cargo_bin("find")
        .expect("found binary")
        .args(["./test_data/simple/subdir", "-fstype"])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty());

    // void fstype
    Command::cargo_bin("find")
        .expect("found binary")
        .args(["./test_data/simple/subdir", "-fstype", " "])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());

    let path = Path::new("./test_data/links");
    let empty_cache = RefCell::new(None);
    let target_fs_type = get_file_system_type(path, &empty_cache).unwrap();

    // working with broken links
    Command::cargo_bin("find")
        .expect("found binary")
        .args(["./test_data/links", "-fstype", &target_fs_type])
        .assert()
        .success()
        .stdout(predicate::str::contains("./test_data/links/link-missing"))
        .stderr(predicate::str::is_empty());
}

#[test]
#[serial(working_dir)]
fn find_samefile() {
    use std::fs;

    // remove file if hard link file exist.
    // But you can't delete a file that doesn't exist,
    // so ignore the error returned here.
    let _ = fs::remove_file("test_data/links/hard_link");
    fs::hard_link("test_data/links/abbbc", "test_data/links/hard_link").unwrap();

    Command::cargo_bin("find")
        .expect("found binary")
        .args([
            "./test_data/links/abbbc",
            "-samefile",
            "./test_data/links/hard_link",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("./test_data/links/abbbc"))
        .stderr(predicate::str::is_empty());

    // test . path
    Command::cargo_bin("find")
        .expect("found binary")
        .args([".", "-samefile", "."])
        .assert()
        .success()
        .stdout(predicate::str::contains("."))
        .stderr(predicate::str::is_empty());

    Command::cargo_bin("find")
        .expect("found binary")
        .args([".", "-samefile", "./test_data/links/abbbc"])
        .assert()
        .success()
        .stdout(predicate::str::contains(fix_up_slashes(
            "./test_data/links/abbbc",
        )))
        .stderr(predicate::str::is_empty());

    // test not exist file
    Command::cargo_bin("find")
        .expect("found binary")
        .args([".", "-samefile", "./test_data/links/not-exist-file"])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("not-exist-file"));

    fs::remove_file("test_data/links/hard_link").unwrap();
}

#[test]
#[serial(working_dir)]
fn find_noleaf() {
    Command::cargo_bin("find")
        .expect("found binary")
        .args(["test_data/simple/subdir", "-noleaf"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test_data/simple/subdir"))
        .stderr(predicate::str::is_empty());
}

#[test]
#[serial(working_dir)]
fn find_daystart() {
    Command::cargo_bin("find")
        .expect("found binary")
        .args(["./test_data/simple/subdir", "-daystart", "-mtime", "0"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    // twice -daystart should be matched
    Command::cargo_bin("find")
        .expect("found binary")
        .args([
            "./test_data/simple/subdir",
            "-daystart",
            "-daystart",
            "-mtime",
            "1",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
#[serial(working_dir)]
fn find_fprinter() {
    let printer = ["fprint", "fprint0"];

    for p in printer.iter() {
        let _ = fs::remove_file(format!("test_data/find_{p}"));

        Command::cargo_bin("find")
            .expect("found binary")
            .args([
                "test_data/simple",
                format!("-{p}").as_str(),
                format!("test_data/find_{p}").as_str(),
            ])
            .assert()
            .success()
            .stdout(predicate::str::is_empty())
            .stderr(predicate::str::is_empty());

        // read test_data/find_fprint
        let mut f = File::open(format!("test_data/find_{p}")).unwrap();
        let mut contents = String::new();
        f.read_to_string(&mut contents).unwrap();
        assert!(contents.contains("test_data/simple"));

        let _ = fs::remove_file(format!("test_data/find_{p}"));
    }
}

#[test]
#[serial(working_dir)]
fn find_follow() {
    Command::cargo_bin("find")
        .expect("found binary")
        .args(["test_data/links/link-f", "-follow"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test_data/links/link-f"))
        .stderr(predicate::str::is_empty());
}

#[test]
#[serial(working_dir)]
fn find_fprintf() {
    let _ = fs::remove_file("test_data/find_fprintf");

    Command::cargo_bin("find")
        .expect("found binary")
        .args([
            "test_data/simple",
            "-fprintf",
            "test_data/find_fprintf",
            "%h %H %p %P",
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());

    // read test_data/find_fprintf
    let mut f = File::open("test_data/find_fprintf").unwrap();
    let mut contents = String::new();
    f.read_to_string(&mut contents).unwrap();
    assert!(contents.contains("test_data/simple"));

    let _ = fs::remove_file("test_data/find_fprintf");
}

#[test]
#[serial(working_dir)]
fn find_ls() {
    Command::cargo_bin("find")
        .expect("found binary")
        .args(["./test_data/simple/subdir", "-ls"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}
