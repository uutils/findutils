// Copyright 2021 Chad Williamson <chad@dahc.us>
//
// Use of this source code is governed by an MIT-style license that can be
// found in the LICENSE file or at https://opensource.org/licenses/MIT.

// Integration tests for the find command using the uutests framework.

use regex::Regex;
use std::fs::{self, File};
use std::io::ErrorKind;
use std::io::{Read, Write};
use std::path::Path;
use tempfile::Builder;
use uutests::util::TestScenario;

#[cfg(unix)]
use std::os::unix::fs::symlink;

#[cfg(windows)]
use std::os::windows::fs::{symlink_dir, symlink_file};

use common::test_helpers::fix_up_slashes;

mod common;

/// Returns a UCommand for `find` with the working directory set to the
/// repository root, so that tests using relative `test_data/` paths work.
fn ucmd() -> uutests::util::UCommand {
    let ts = TestScenario::new("find");
    let mut cmd = ts.cmd(env!("CARGO_BIN_EXE_find"));
    cmd.current_dir(env!("CARGO_MANIFEST_DIR"));
    cmd
}

// Variants of fix_up_slashes that properly escape the forward slashes for
// use in a regex.
#[cfg(windows)]
fn fix_up_regex_slashes(re: &str) -> String {
    re.replace("/", "\\\\")
}

#[cfg(not(windows))]
fn fix_up_regex_slashes(re: &str) -> String {
    re.to_owned()
}

#[test]
fn no_args() {
    ucmd().succeeds().no_stderr().stdout_contains("test_data");
}

#[test]
fn two_matchers_both_match() {
    ucmd()
        .args(&["-type", "d", "-name", "test_data"])
        .succeeds()
        .no_stderr()
        .stdout_contains("test_data");
}

#[test]
fn two_matchers_one_matches() {
    ucmd()
        .args(&["-type", "f", "-name", "test_data"])
        .succeeds()
        .no_output();
}

#[test]
fn multiple_matcher_success() {
    ucmd()
        .args(&["-type", "f,d,l", "-name", "abbbc"])
        .succeeds()
        .no_stderr()
        .stdout_contains("abbbc");

    ucmd()
        .args(&["-xtype", "f,d,l", "-name", "abbbc"])
        .succeeds()
        .no_stderr()
        .stdout_contains("abbbc");
}

#[test]
fn multiple_matcher_failure() {
    ucmd()
        .args(&["-type", "fd", "-name", "abbb"])
        .fails()
        .stderr_contains("Must separate multiple arguments")
        .no_stdout();

    ucmd()
        .args(&["-type", "f,", "-name", "abbb"])
        .fails()
        .stderr_contains("list is ending on: ','")
        .no_stdout();

    ucmd()
        .args(&["-type", "f,f", "-name", "abbb"])
        .fails()
        .stderr_contains("Duplicate file type")
        .no_stdout();

    ucmd()
        .args(&["-type", "", "-name", "abbb"])
        .fails()
        .stderr_contains("should contain at least one letter")
        .no_stdout();

    ucmd()
        .args(&["-type", "x,y", "-name", "abbb"])
        .fails()
        .stderr_contains("Unrecognised type argument")
        .no_stdout();

    // x-type tests below
    ucmd()
        .args(&["-xtype", "fd", "-name", "abbb"])
        .fails()
        .stderr_contains("Must separate multiple arguments")
        .no_stdout();

    ucmd()
        .args(&["-xtype", "f,", "-name", "abbb"])
        .fails()
        .stderr_contains("list is ending on: ','")
        .no_stdout();

    ucmd()
        .args(&["-xtype", "f,f", "-name", "abbb"])
        .fails()
        .stderr_contains("Duplicate file type")
        .no_stdout();

    ucmd()
        .args(&["-xtype", "", "-name", "abbb"])
        .fails()
        .stderr_contains("should contain at least one letter")
        .no_stdout();

    ucmd()
        .args(&["-xtype", "x,y", "-name", "abbb"])
        .fails()
        .stderr_contains("Unrecognised type argument")
        .no_stdout();
}

#[test]
fn files0_empty_file() {
    ucmd()
        .args(&["-files0-from", "./test_data/simple/abbbc"])
        .succeeds()
        .no_output();
}

#[test]
fn files0_file_basic_success() {
    ucmd()
        .args(&["-files0-from", "./test_data/simple/abbbc"])
        .succeeds()
        .no_output();

    #[cfg(unix)]
    {
        let temp_dir = Builder::new().prefix("find_cmd_").tempdir().unwrap();
        let test_file = temp_dir.path().join("test_files0");
        let mut file = File::create(&test_file).expect("created test file");
        file.write_all(b"./test_data/\0./test_data/simple/\0")
            .expect("file write error");

        ucmd()
            .args(&["-files0-from", &test_file.display().to_string()])
            .succeeds()
            .no_stderr()
            .stdout_contains("/test_data/");
    }
}

#[test]
fn files0_empty_pipe() {
    ucmd()
        .args(&["-files0-from", "-"])
        .pipe_in(b"" as &[u8])
        .succeeds()
        .no_output();
}

#[test]
fn files0_pipe_basic() {
    ucmd()
        .pipe_in(b"./test_data/simple\0./test_data/links" as &[u8])
        .args(&["-files0-from", "-"])
        .succeeds()
        .no_stderr()
        .stdout_contains("./test_data/");
}

#[test]
fn files0_pipe_double_nul() {
    ucmd()
        .pipe_in(b"./test_data/simple\0\0./test_data/links" as &[u8])
        .args(&["-files0-from", "-"])
        .succeeds()
        .stderr_contains("invalid zero-length file name")
        .stdout_contains("./test_data/");
}

#[test]
fn files0_no_file() {
    #[cfg(unix)]
    {
        ucmd()
            .args(&["-files0-from", "xyz.nonexistentFile"])
            .fails()
            .stderr_contains("No such file or directory")
            .no_stdout();
    }
    #[cfg(windows)]
    {
        ucmd()
            .args(&["-files0-from", "xyz.nonexistantFile"])
            .fails()
            .stderr_contains("The system cannot find the file specified.")
            .no_stdout();
    }
}

#[test]
fn files0_basic() {
    ucmd()
        .arg("-files0-from")
        .fails()
        .stderr_contains("missing argument to -files0-from")
        .no_stdout();
}

#[test]
fn matcher_with_side_effects_at_end() {
    let temp_dir = Builder::new().prefix("find_cmd_").tempdir().unwrap();

    let temp_dir_path = temp_dir.path().to_str().unwrap();
    let test_file = temp_dir.path().join("test");
    File::create(&test_file).expect("created test file");

    ucmd()
        .args(&[temp_dir_path, "-name", "test", "-delete"])
        .succeeds()
        .no_output();

    assert!(!test_file.exists(), "test file should be deleted");
    assert!(temp_dir.path().exists(), "temp dir should NOT be deleted");
}

#[test]
fn matcher_with_side_effects_in_front() {
    let temp_dir = Builder::new().prefix("find_cmd_").tempdir().unwrap();

    let temp_dir_path = temp_dir.path().to_str().unwrap();
    let test_file = temp_dir.path().join("test");
    File::create(&test_file).expect("created test file");

    ucmd()
        .args(&[temp_dir_path, "-delete", "-name", "test"])
        .succeeds()
        .no_output();

    assert!(!test_file.exists(), "test file should be deleted");
    assert!(!temp_dir.path().exists(), "temp dir should also be deleted");
}

// This could be covered by a unit test in principle... in practice, changing
// the working dir can't be done safely in unit tests unless `--test-threads=1`
// or `serial` goes everywhere, and it doesn't seem possible to get an
// appropriate `walkdir::DirEntry` for "." without actually changing dirs
// (or risking deletion of the repo itself).
#[test]
fn delete_on_dot_dir() {
    let temp_dir = Builder::new().prefix("example").tempdir().unwrap();

    // "." should be matched (confirmed by the print), but not deleted.
    ucmd()
        .current_dir(temp_dir.path())
        .args(&[".", "-delete", "-print"])
        .succeeds()
        .no_stderr()
        .stdout_only(".\n");

    assert!(temp_dir.path().exists(), "temp dir should still exist");
}

#[test]
fn regex_types() {
    let temp_dir = Builder::new().prefix("find_cmd_").tempdir().unwrap();

    let temp_dir_path = temp_dir.path().to_str().unwrap();
    let test_file = temp_dir.path().join("teeest");
    File::create(test_file).expect("created test file");

    ucmd()
        .arg(temp_dir_path)
        .arg("-regex")
        .arg(fix_up_regex_slashes(".*/tE+st"))
        .succeeds()
        .no_output();

    ucmd()
        .arg(temp_dir_path)
        .arg("-iregex")
        .arg(fix_up_regex_slashes(".*/tE+st"))
        .succeeds()
        .no_stderr()
        .stdout_contains("teeest");

    ucmd()
        .arg(temp_dir_path)
        .args(&["-regextype", "posix-basic", "-regex"])
        .arg(fix_up_regex_slashes(r".*/te\{1,3\}st"))
        .succeeds()
        .no_stderr()
        .stdout_contains("teeest");

    ucmd()
        .arg(temp_dir_path)
        .args(&["-regextype", "posix-extended", "-regex"])
        .arg(fix_up_regex_slashes(".*/te{1,3}st"))
        .succeeds()
        .no_stderr()
        .stdout_contains("teeest");

    ucmd()
        .arg(temp_dir_path)
        .args(&["-regextype", "ed", "-regex"])
        .arg(fix_up_regex_slashes(r".*/te\{1,3\}st"))
        .succeeds()
        .no_stderr()
        .stdout_contains("teeest");

    ucmd()
        .arg(temp_dir_path)
        .args(&["-regextype", "sed", "-regex"])
        .arg(fix_up_regex_slashes(r".*/te\{1,3\}st"))
        .succeeds()
        .no_stderr()
        .stdout_contains("teeest");
}

#[test]
fn empty_files() {
    let temp_dir = Builder::new().prefix("find_cmd_").tempdir().unwrap();
    let temp_dir_path = temp_dir.path().to_str().unwrap();

    ucmd()
        .args(&[temp_dir_path, "-empty"])
        .succeeds()
        .no_stderr()
        .stdout_only(fix_up_slashes(&format!("{temp_dir_path}\n")));

    let test_file_path = temp_dir.path().join("test");
    let mut test_file = File::create(&test_file_path).unwrap();

    ucmd()
        .args(&[temp_dir_path, "-empty"])
        .succeeds()
        .no_stderr()
        .stdout_only(fix_up_slashes(&format!(
            "{}\n",
            test_file_path.to_string_lossy()
        )));

    let subdir_path = temp_dir.path().join("subdir");
    std::fs::create_dir(&subdir_path).unwrap();

    ucmd()
        .args(&[temp_dir_path, "-empty", "-sorted"])
        .succeeds()
        .no_stderr()
        .stdout_only(fix_up_slashes(&format!(
            "{}\n{}\n",
            subdir_path.to_string_lossy(),
            test_file_path.to_string_lossy()
        )));

    write!(test_file, "x").unwrap();
    test_file.sync_all().unwrap();

    ucmd()
        .args(&[temp_dir_path, "-empty", "-sorted"])
        .succeeds()
        .no_stderr()
        .stdout_only(fix_up_slashes(&format!(
            "{}\n",
            subdir_path.to_string_lossy(),
        )));
}

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

    ucmd()
        .arg(fix_up_slashes("./test_data/simple"))
        .args(&["-sorted", "-printf", "%f %d %h %H %p %P %y\n"])
        .succeeds()
        .no_stderr()
        .stdout_only(fix_up_slashes(
            "simple 0 ./test_data ./test_data/simple \
            ./test_data/simple  d\n\
            abbbc 1 ./test_data/simple ./test_data/simple \
            ./test_data/simple/abbbc abbbc f\n\
            subdir 1 ./test_data/simple ./test_data/simple \
            ./test_data/simple/subdir subdir d\n\
            ABBBC 2 ./test_data/simple/subdir ./test_data/simple \
            ./test_data/simple/subdir/ABBBC subdir/ABBBC f\n",
        ));

    fs::create_dir_all("a").expect("Failed to create directory 'a'");
    let result = ucmd().args(&["a", "-printf", "%A+"]).succeeds();

    let output_str = result.stdout_str();
    let re = Regex::new(r"^\d{4}-\d{2}-\d{2}\+\d{2}:\d{2}:\d{2}\.\d{9}0$")
        .expect("Failed to compile regex");
    assert!(
        re.is_match(output_str.trim()),
        "Output did not match expected timestamp format"
    );

    ucmd()
        .arg(fix_up_slashes("./test_data/links"))
        .args(&["-sorted", "-type", "l", "-printf", "%f %l %y %Y\n"])
        .succeeds()
        .no_stderr()
        .stdout_only(
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
        );
}

#[cfg(unix)]
#[test]
fn find_perm() {
    ucmd().args(&["-perm", "+rwx"]).succeeds();
    ucmd().args(&["-perm", "u+rwX"]).succeeds();
    ucmd().args(&["-perm", "u=g"]).succeeds();
}

#[cfg(unix)]
#[test]
fn find_inum() {
    use std::os::unix::fs::MetadataExt;

    let inum = fs::metadata("test_data/simple/abbbc")
        .expect("metadata for abbbc")
        .ino()
        .to_string();

    ucmd()
        .args(&["test_data", "-inum", &inum])
        .succeeds()
        .no_stderr()
        .stdout_contains("abbbc");
}

#[cfg(not(unix))]
#[test]
fn find_inum() {
    ucmd()
        .args(&["test_data", "-inum", "1"])
        .fails()
        .stderr_contains("not available on this platform")
        .no_stdout();
}

#[cfg(unix)]
#[test]
fn find_links() {
    ucmd()
        .args(&["test_data", "-links", "1"])
        .succeeds()
        .no_stderr()
        .stdout_contains("abbbc");
}

#[cfg(not(unix))]
#[test]
fn find_links() {
    ucmd()
        .args(&["test_data", "-links", "1"])
        .fails()
        .stderr_contains("not available on this platform")
        .no_stdout();
}

#[test]
fn find_mount_xdev() {
    // Make sure that -mount/-xdev doesn't prune unexpectedly.
    // TODO: Test with a mount point in the search.
    ucmd()
        .args(&["test_data", "-mount"])
        .succeeds()
        .no_stderr()
        .stdout_contains("abbbc");

    ucmd()
        .args(&["test_data", "-xdev"])
        .succeeds()
        .no_stderr()
        .stdout_contains("abbbc");
}

#[test]
fn find_accessible() {
    ucmd()
        .args(&["test_data", "-readable"])
        .succeeds()
        .no_stderr()
        .stdout_contains("abbbc");

    ucmd()
        .args(&["test_data", "-writable"])
        .succeeds()
        .no_stderr()
        .stdout_contains("abbbc");

    #[cfg(unix)]
    ucmd()
        .args(&["test_data", "-executable"])
        .succeeds()
        .no_stderr()
        .stdout_does_not_contain("abbbc");
}

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
            ucmd()
                .args(&["./test_data/simple", flag, arg])
                .succeeds()
                .no_stderr();
        });

        exception_args.iter().for_each(|arg| {
            ucmd().args(&[".", flag, arg]).fails().no_stdout();
        });
    });
}

#[test]
fn expression_empty_parentheses() {
    ucmd()
        .args(&["-true", "(", ")"])
        .fails()
        .stderr_contains("empty parentheses are not allowed")
        .no_stdout();
}

#[test]
#[cfg(unix)]
fn find_with_user_predicate() {
    // Considering the different test environments,
    // the test code can only use a specific default user to perform the test,
    // such as the root user on Linux.
    ucmd()
        .args(&["test_data", "-user", "root"])
        .succeeds()
        .no_stderr();

    ucmd()
        .args(&["test_data", "-user", ""])
        .fails()
        .stderr_contains("empty")
        .no_stdout();

    ucmd()
        .args(&["test_data", "-user", " "])
        .fails()
        .stderr_contains("invalid user name or UID argument to -user")
        .no_stdout();
}

#[test]
fn find_with_nouser_predicate() {
    ucmd()
        .args(&["test_data", "-nouser"])
        .succeeds()
        .no_stdout()
        .no_stderr();
}

#[test]
#[cfg(unix)]
fn find_with_uid_predicate() {
    use std::os::unix::fs::MetadataExt;

    let uid = Path::new("./test_data")
        .metadata()
        .unwrap()
        .uid()
        .to_string();

    ucmd()
        .args(&["test_data", "-uid", &uid])
        .succeeds()
        .no_stderr();
}

#[test]
fn find_with_group_predicate() {
    // Considering the different test environments,
    // the test code can only use a specific default user group for the test,
    // such as the root user group on Linux.
    #[cfg(target_os = "linux")]
    ucmd()
        .args(&["test_data", "-group", "root"])
        .succeeds()
        .no_stderr();

    #[cfg(target_os = "macos")]
    ucmd()
        .args(&["test_data", "-group", "staff"])
        .succeeds()
        .no_stderr();

    ucmd()
        .args(&["test_data", "-group", ""])
        .fails()
        .stderr_contains("empty")
        .no_stdout();

    ucmd()
        .args(&["test_data", "-group", " "])
        .fails()
        .stderr_contains("invalid group name or GID argument to -group:")
        .no_stdout();
}

#[test]
fn find_with_nogroup_predicate() {
    ucmd()
        .args(&["test_data", "-nogroup"])
        .succeeds()
        .no_stdout()
        .no_stderr();
}

#[test]
#[cfg(unix)]
fn find_with_gid_predicate() {
    use std::os::unix::fs::MetadataExt;

    let gid = Path::new("./test_data")
        .metadata()
        .unwrap()
        .gid()
        .to_string();

    ucmd()
        .args(&["test_data", "-gid", &gid])
        .succeeds()
        .no_stderr();
}

#[test]
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
            ucmd()
                .args(&[
                    "./test_data/simple/subdir",
                    arg,
                    "./test_data/simple/subdir/ABBBC",
                ])
                .succeeds()
                .no_stderr();
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
            ucmd()
                .args(&["./test_data/simple/subdir", arg, time])
                .succeeds()
                .no_stderr();
        }
    }
}

#[test]
fn find_age_range() {
    let args = ["-amin", "-cmin", "-mmin"];
    let times = ["-60", "-120", "-240", "+60", "+120", "+240"];
    let time_strings = [
        "\"-60\"", "\"-120\"", "\"-240\"", "\"-60\"", "\"-120\"", "\"-240\"",
    ];

    for arg in args {
        for time in times {
            ucmd().args(&["test_data/simple", arg, time]).succeeds();
        }
    }

    for arg in args {
        for time_string in time_strings {
            ucmd()
                .args(&["test_data/simple", arg, time_string])
                .fails()
                .stderr_contains(
                    "find: Expected a decimal integer (with optional + or - prefix) argument to",
                )
                .no_stdout();
        }
    }
}

#[test]
#[cfg(unix)]
fn find_fs() {
    use findutils::find::matchers::fs::get_file_system_type;
    use std::cell::RefCell;

    let path = Path::new("./test_data/simple/subdir");
    let empty_cache = RefCell::new(None);
    let target_fs_type = get_file_system_type(path, &empty_cache).unwrap();

    // match fs type
    ucmd()
        .args(&["./test_data/simple/subdir", "-fstype", &target_fs_type])
        .succeeds()
        .stdout_contains("./test_data/simple/subdir")
        .no_stderr();

    // not match fs type
    ucmd()
        .args(&[
            "./test_data/simple/subdir",
            "-fstype",
            format!("{} foo", target_fs_type).as_str(),
        ])
        .succeeds()
        .no_output();

    // not contain fstype text.
    ucmd()
        .args(&["./test_data/simple/subdir", "-fstype"])
        .fails()
        .no_stdout();

    // void fstype
    ucmd()
        .args(&["./test_data/simple/subdir", "-fstype", " "])
        .succeeds()
        .no_output();

    let path = Path::new("./test_data/links");
    let empty_cache = RefCell::new(None);
    let target_fs_type = get_file_system_type(path, &empty_cache).unwrap();

    // working with broken links
    ucmd()
        .args(&["./test_data/links", "-fstype", &target_fs_type])
        .succeeds()
        .stdout_contains("./test_data/links/link-missing")
        .no_stderr();
}

#[test]
fn find_samefile() {
    let temp_dir = Builder::new().prefix("find_samefile_").tempdir().unwrap();
    let temp_dir_path = temp_dir.path().to_str().unwrap();
    let test_file = temp_dir.path().join("abbbc");
    let hard_link = temp_dir.path().join("hard_link");
    fs::copy("test_data/links/abbbc", &test_file).unwrap();
    fs::hard_link(&test_file, &hard_link).unwrap();

    let test_file_str = test_file.to_str().unwrap();
    let hard_link_str = hard_link.to_str().unwrap();
    let not_exist = temp_dir.path().join("not-exist-file");
    let not_exist_str = not_exist.to_str().unwrap();

    ucmd()
        .args(&[test_file_str, "-samefile", hard_link_str])
        .succeeds()
        .stdout_contains(test_file_str)
        .no_stderr();

    // test . path
    ucmd()
        .args(&[temp_dir_path, "-samefile", temp_dir_path])
        .succeeds()
        .stdout_contains(temp_dir_path)
        .no_stderr();

    ucmd()
        .args(&[temp_dir_path, "-samefile", test_file_str])
        .succeeds()
        .stdout_contains(fix_up_slashes(test_file_str))
        .no_stderr();

    // test not exist file
    ucmd()
        .args(&[temp_dir_path, "-samefile", not_exist_str])
        .fails()
        .no_stdout()
        .stderr_contains("not-exist-file");
}

#[test]
fn find_noleaf() {
    ucmd()
        .args(&["test_data/simple/subdir", "-noleaf"])
        .succeeds()
        .stdout_contains("test_data/simple/subdir")
        .no_stderr();
}

#[test]
fn find_daystart() {
    ucmd()
        .args(&["./test_data/simple/subdir", "-daystart", "-mtime", "0"])
        .succeeds()
        .no_stderr();

    // twice -daystart should be matched
    ucmd()
        .args(&[
            "./test_data/simple/subdir",
            "-daystart",
            "-daystart",
            "-mtime",
            "1",
        ])
        .succeeds()
        .no_stderr();
}

#[test]
fn find_fprinter() {
    let temp_dir = Builder::new().prefix("find_fprinter_").tempdir().unwrap();
    let printer = ["fprint", "fprint0"];

    for p in &printer {
        let out_file = temp_dir.path().join(format!("find_{p}"));
        let out_file_str = out_file.to_str().unwrap();

        ucmd()
            .args(&["test_data/simple", format!("-{p}").as_str(), out_file_str])
            .succeeds()
            .no_output();

        // read test_data/find_fprint
        let mut f = File::open(&out_file).unwrap();
        let mut contents = String::new();
        f.read_to_string(&mut contents).unwrap();
        assert!(contents.contains("test_data/simple"));
    }
}

#[test]
fn find_follow() {
    ucmd()
        .args(&["test_data/links/link-f", "-follow"])
        .succeeds()
        .stdout_contains("test_data/links/link-f")
        .no_stderr();
}

#[test]
fn find_fprintf() {
    let temp_dir = Builder::new().prefix("find_fprintf_").tempdir().unwrap();
    let out_file = temp_dir.path().join("find_fprintf");
    let out_file_str = out_file.to_str().unwrap();

    ucmd()
        .args(&["test_data/simple", "-fprintf", out_file_str, "%h %H %p %P"])
        .succeeds()
        .no_output();

    // read test_data/find_fprintf
    let mut f = File::open(&out_file).unwrap();
    let mut contents = String::new();
    f.read_to_string(&mut contents).unwrap();
    assert!(contents.contains("test_data/simple"));
}

#[test]
fn find_ls() {
    ucmd()
        .args(&["./test_data/simple/subdir", "-ls"])
        .succeeds()
        .no_stderr();
}

#[test]
#[cfg(unix)]
fn find_slashes() {
    ucmd()
        .args(&["///", "-maxdepth", "0", "-name", "/"])
        .succeeds()
        .no_stderr();
}

// -ok / -okdir integration tests
//
// These tests use pipe_in() to supply the user's response.  Because pipe_in()
// makes stdin a pipe, std::io::stdin().is_terminal() returns false inside the
// find subprocess, so StandardDependencies::confirm reads from stdin directly
// instead of opening /dev/tty.  No special environment variable is needed.

#[test]
fn find_ok_yes_runs_command() {
    // When the user answers "y", -ok should run the command and print output.
    ucmd()
        .args(&[
            "test_data/simple",
            "-maxdepth",
            "1",
            "-name",
            "abbbc",
            "-ok",
            "echo",
            "{}",
            ";",
        ])
        // Pipe the affirmative response for the single file found.
        .pipe_in("y\n")
        .succeeds()
        .stderr_contains("< echo") // prompt appeared
        .stdout_contains("abbbc"); // echo ran
}

#[test]
fn find_ok_no_skips_command() {
    // When the user answers "n", -ok should not run the command.
    // The expression is false so no output is produced, but find exits 0.
    ucmd()
        .args(&[
            "test_data/simple",
            "-maxdepth",
            "1",
            "-name",
            "abbbc",
            "-ok",
            "echo",
            "{}",
            ";",
        ])
        .pipe_in("n\n")
        .succeeds()
        .stderr_contains("< echo") // prompt still appeared
        .no_stdout(); // but echo was not run
}

#[test]
fn find_ok_prompt_format() {
    // The prompt should follow GNU find's "< executable args... >? " format.
    ucmd()
        .args(&[
            "test_data/simple",
            "-maxdepth",
            "1",
            "-name",
            "abbbc",
            "-ok",
            "echo",
            "{}",
            ";",
        ])
        .pipe_in("n\n")
        .succeeds()
        .stderr_contains(format!(
            "< echo {} >? ",
            Path::new("test_data/simple")
                .join("abbbc")
                .to_string_lossy()
        ));
}

#[test]
fn find_ok_empty_response_declines() {
    // An empty line (just Enter) should be treated as decline.
    ucmd()
        .args(&[
            "test_data/simple",
            "-maxdepth",
            "1",
            "-name",
            "abbbc",
            "-ok",
            "echo",
            "{}",
            ";",
        ])
        .pipe_in("\n")
        .succeeds()
        .no_stdout();
}

#[test]
fn find_ok_accepts_y_variants() {
    // "Y", "yes", and " y" (leading whitespace) should all be accepted.
    for response in &["Y\n", "yes\n", " y\n"] {
        ucmd()
            .args(&[
                "test_data/simple",
                "-maxdepth",
                "1",
                "-name",
                "abbbc",
                "-ok",
                "echo",
                "{}",
                ";",
            ])
            .pipe_in(*response)
            .succeeds()
            .stdout_contains("abbbc");
    }
}

#[test]
fn find_okdir_yes_runs_command() {
    // -okdir should run the command in the file's parent directory.
    ucmd()
        .args(&[
            "test_data/simple",
            "-maxdepth",
            "1",
            "-name",
            "abbbc",
            "-okdir",
            "echo",
            "{}",
            ";",
        ])
        .pipe_in("y\n")
        .succeeds()
        .stderr_contains("< echo")
        .stdout_contains(fix_up_slashes("./abbbc"));
}

#[test]
fn find_ok_missing_semicolon() {
    // -ok without a closing ';' should be an error (just like -exec).
    ucmd()
        .args(&["test_data/simple", "-ok", "echo", "{}"])
        .pipe_in("")
        .fails()
        .stderr_contains("missing argument to -ok")
        .no_stdout();
}
