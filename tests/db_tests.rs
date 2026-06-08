// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

mod common;

use std::process::Command;

use assert_cmd::{assert::OutputAssertExt, cargo::CommandCargoExt};
use rstest::rstest;

const DB_FLAG: &str = "--database=test_data/db/test_data_db";
const INVALID_DB_FLAG: &str = "--database=test_data/db/invalid_db";
const OLD_DB_FLAG: &str = "--database=test_data/db/old_db";

#[test]
fn test_locate_no_matches() {
    Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .args(["usr", DB_FLAG])
        .assert()
        .failure();
}

#[test]
fn test_locate_match() {
    Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .args(["test_data", DB_FLAG])
        .assert()
        .success();
}

#[test]
fn test_locate_no_matches_basename() {
    Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .args(["test_data1234567890", "--basename", DB_FLAG])
        .assert()
        .failure();
}

#[test]
fn test_locate_match_basename() {
    Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .args(["abbbc", "--basename", DB_FLAG])
        .assert()
        .success();
}

#[test]
fn test_locate_existing() {
    Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .args(["abbbc", "--existing", DB_FLAG])
        .assert()
        .success();
}

#[test]
fn test_locate_non_existing() {
    Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .args(["abbbc", "--non-existing", DB_FLAG])
        .assert()
        .failure();
}

#[test]
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
fn test_locate_all() {
    Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .args(["abb", "bbc", "--all", DB_FLAG])
        .assert()
        .success();
}

#[test]
fn test_locate_all_regex() {
    Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .args(["abb", "b*c", "--all", "--regex", DB_FLAG])
        .assert()
        .success();
}

#[test]
fn test_locate_invalid_db() {
    Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .args(["test_data", INVALID_DB_FLAG])
        .assert()
        .failure();
}

#[test]
fn test_locate_outdated_db() {
    Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .args(["test_data", OLD_DB_FLAG])
        .assert()
        .success();
}

#[test]
fn test_locate_print_help() {
    Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn test_locate_invalid_flag() {
    Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .arg("--unknown")
        .assert()
        .failure();
}

// an un-compilable regex should be reported as an error rather than silently matching nothing
#[test]
fn test_locate_invalid_regex() {
    Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .args(["[", "--regex", DB_FLAG])
        .assert()
        .failure();
}

#[test]
fn test_updatedb() {
    let tmp = tempfile::tempdir().unwrap();
    Command::cargo_bin("updatedb")
        .expect("couldn't find updatedb binary")
        .args([
            "--localpaths=./test_data".to_string(),
            format!("--output={}", tmp.path().join("db").display()),
        ])
        .assert()
        .success();
}

#[test]
fn test_updatedb_invalid_flag() {
    Command::cargo_bin("updatedb")
        .expect("couldn't find updatedb binary")
        .args(["--unknown"])
        .assert()
        .failure();
}

// empty prunefs/prunepaths must not produce an invalid find expression (e.g. an empty `( )` group)
#[test]
fn test_updatedb_empty_prune() {
    let tmp = tempfile::tempdir().unwrap();
    Command::cargo_bin("updatedb")
        .expect("couldn't find updatedb binary")
        .args([
            "--localpaths=./test_data".to_string(),
            format!("--output={}", tmp.path().join("db").display()),
            "--prunefs=".to_string(),
            "--prunepaths=".to_string(),
        ])
        .assert()
        .success();
}

// when the output database can't be created, updatedb must report a clear error naming the path
// and must not leak the raw "(os error N)" suffix
#[test]
fn test_updatedb_output_create_error() {
    let tmp = tempfile::tempdir().unwrap();
    // a path under a non-existent directory can't be created
    let bad_output = tmp.path().join("does-not-exist").join("db");
    let assert = Command::cargo_bin("updatedb")
        .expect("couldn't find updatedb binary")
        .args([
            "--localpaths=./test_data".to_string(),
            format!("--output={}", bad_output.display()),
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("cannot create") && stderr.contains(&bad_output.display().to_string()),
        "stderr did not name the un-creatable output path: {stderr:?}"
    );
    assert!(
        !stderr.contains("os error"),
        "stderr leaked the raw OS error: {stderr:?}"
    );
}

// build a database from a temp tree with updatedb, then query it back with locate. This is the
// only test that exercises the full pipeline (writer + reader) and is platform-independent.
#[test]
fn test_updatedb_locate_roundtrip() {
    // create the tree under the target dir so the path has no spaces (updatedb splits
    // --localpaths on whitespace, mirroring GNU's space-separated convention)
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let marker = "locate_roundtrip_marker";
    std::fs::write(dir.path().join(marker), b"").unwrap();
    let db = dir.path().join("db");

    Command::cargo_bin("updatedb")
        .expect("couldn't find updatedb binary")
        .args([
            format!("--localpaths={}", dir.path().display()),
            format!("--output={}", db.display()),
        ])
        .assert()
        .success();

    let assert = Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .args([marker.to_string(), format!("--database={}", db.display())])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains(marker),
        "locate output did not contain the indexed file: {stdout:?}"
    );
}
