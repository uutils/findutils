// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

mod common;

use std::process::Command;

use assert_cmd::{assert::OutputAssertExt, cargo::CommandCargoExt};
use rstest::rstest;

#[cfg(not(windows))]
const DB_FLAG: &str = "--database=test_data/db/test_data_db";
#[cfg(not(windows))]
const INVALID_DB_FLAG: &str = "--database=test_data/db/invalid_db";
#[cfg(not(windows))]
const OLD_DB_FLAG: &str = "--database=test_data/db/old_db";

#[test]
#[cfg(not(windows))]
fn test_locate_no_matches() {
    Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .args(["usr", DB_FLAG])
        .assert()
        .failure();
}

#[test]
#[cfg(not(windows))]
fn test_locate_match() {
    Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .args(["test_data", DB_FLAG])
        .assert()
        .success();
}

#[test]
#[cfg(not(windows))]
fn test_locate_no_matches_basename() {
    Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .args(["test_data1234567890", "--basename", DB_FLAG])
        .assert()
        .failure();
}

#[test]
#[cfg(not(windows))]
fn test_locate_match_basename() {
    Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .args(["abbbc", "--basename", DB_FLAG])
        .assert()
        .success();
}

#[test]
#[cfg(not(windows))]
fn test_locate_existing() {
    Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .args(["abbbc", "--existing", DB_FLAG])
        .assert()
        .success();
}

#[test]
#[cfg(not(windows))]
fn test_locate_non_existing() {
    Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .args(["abbbc", "--non-existing", DB_FLAG])
        .assert()
        .failure();
}

#[test]
#[cfg(not(windows))]
fn test_locate_statistics() {
    Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .args(["", "--statistics", DB_FLAG])
        .assert()
        .success();
}

#[rstest]
#[case("emacs")]
#[case("grep")]
#[case("posix-basic")]
#[case("posix-extended")]
#[cfg(not(windows))]
fn test_locate_regex(#[case] input: &str) {
    Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .args([
            "abbbc",
            "--regex",
            format!("--regextype={input}").as_str(),
            DB_FLAG,
        ])
        .assert()
        .success();
}

#[test]
#[cfg(not(windows))]
fn test_locate_all() {
    Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .args(["abb", "bbc", "--all", DB_FLAG])
        .assert()
        .success();
}

#[test]
#[cfg(not(windows))]
fn test_locate_all_regex() {
    Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .args(["abb", "b*c", "--all", "--regex", DB_FLAG])
        .assert()
        .success();
}

#[test]
#[cfg(not(windows))]
fn test_locate_invalid_db() {
    Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .args(["test_data", INVALID_DB_FLAG])
        .assert()
        .failure();
}

#[test]
#[cfg(not(windows))]
fn test_locate_outdated_db() {
    Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .args(["test_data", OLD_DB_FLAG])
        .assert()
        .success();
}

#[test]
#[cfg(not(windows))]
fn test_locate_print_help() {
    Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .arg("--help")
        .assert()
        .success();
}

#[test]
#[cfg(not(windows))]
fn test_locate_invalid_flag() {
    Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .arg("--unknown")
        .assert()
        .failure();
}

// an un-compilable regex should be reported as an error rather than silently matching nothing
#[test]
#[cfg(not(windows))]
fn test_locate_invalid_regex() {
    Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .args(["[", "--regex", DB_FLAG])
        .assert()
        .failure();
}

#[test]
#[cfg(not(windows))]
fn test_updatedb() {
    Command::cargo_bin("updatedb")
        .expect("couldn't find updatedb binary")
        .args(["--localpaths=./test_data", "--output=/dev/null"])
        .assert()
        .success();
}

#[test]
#[cfg(not(windows))]
fn test_updatedb_invalid_flag() {
    Command::cargo_bin("updatedb")
        .expect("couldn't find updatedb binary")
        .args(["--unknown"])
        .assert()
        .failure();
}

// empty prunefs/prunepaths must not produce an invalid find expression (e.g. an empty `( )` group)
#[test]
#[cfg(not(windows))]
fn test_updatedb_empty_prune() {
    Command::cargo_bin("updatedb")
        .expect("couldn't find updatedb binary")
        .args([
            "--localpaths=./test_data",
            "--output=/dev/null",
            "--prunefs=",
            "--prunepaths=",
        ])
        .assert()
        .success();
}
