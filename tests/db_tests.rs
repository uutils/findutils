// Copyright 2017 Google Inc.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

mod common;

use std::{fs::File, io, process::Command};

use assert_cmd::{assert::OutputAssertExt, cargo::CommandCargoExt};
use rstest::{fixture, rstest};

#[fixture]
fn add_special_files() -> io::Result<()> {
    File::create("test_data/db/abc def")?;
    File::create("test_data/db/abc\ndef")?;
    File::create("test_data/db/✨sparkles✨")?;
    Ok(())
}

#[cfg(not(windows))]
const DB_FLAG: &str = "--database=test_data_db";

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

#[rstest]
#[cfg(not(windows))]
fn test_locate_statistics(add_special_files: io::Result<()>) {
    if add_special_files.is_ok() {
        Command::cargo_bin("locate")
            .expect("couldn't find locate binary")
            .args(["abbbc", "--statistics", DB_FLAG])
            .assert()
            .success();
    }
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

#[rstest]
#[cfg(not(windows))]
fn test_updatedb(_add_special_files: io::Result<()>) {
    Command::cargo_bin("updatedb")
        .expect("couldn't find updatedb binary")
        .args(["--localpaths=./test_data", "--output=/dev/null"])
        .assert()
        .success();
}
