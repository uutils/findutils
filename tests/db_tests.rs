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

#[rstest]
#[case("--all")]
#[case("-A")]
#[case("-a")]
fn test_locate_all(#[case] flag: &str) {
    Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .args([DB_FLAG, flag, "abb", "bbc"])
        .assert()
        .success();
}

#[rstest]
#[case("--all")]
#[case("-A")]
#[case("-a")]
fn test_locate_all_requires_every_pattern(#[case] flag: &str) {
    // only "abb" matches an entry; with --all/-A/-a every pattern has to match
    Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .args([DB_FLAG, flag, "zzzz_nomatch", "abb"])
        .assert()
        .failure();
}

#[test]
fn test_locate_any_pattern_without_all() {
    // same patterns as above, but without --all a single matching pattern is enough
    Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .args([DB_FLAG, "zzzz_nomatch", "abb"])
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
fn test_locate_ignore_case_plain() {
    let assert = Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .args(["-i", "-c", "abbbc", DB_FLAG])
        .assert()
        .success();
    assert_eq!(String::from_utf8_lossy(&assert.get_output().stdout), "3\n");
}

#[test]
fn test_locate_ignore_case_plain_all() {
    let assert = Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .args(["--ignore-case", "--all", "-c", "ABB", "bbc", DB_FLAG])
        .assert()
        .success();
    assert_eq!(String::from_utf8_lossy(&assert.get_output().stdout), "3\n");
}

#[test]
fn test_locate_invalid_db() {
    let assert = Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .args(["test_data", INVALID_DB_FLAG])
        .assert()
        .failure();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    // the second record has a shared prefix count of -2, so only the first entry is valid
    assert_eq!(stdout, "./test_data\n");
    assert!(stderr.contains("corrupt or invalid"), "{stderr}");
}

// a LOCATE02 shared prefix count must stay within the previous entry (#967)
#[rstest]
#[case::negative(b"\0LOCATE02\0\x00/srv\0\xfe/q\0", "/srv\n")]
#[case::past_end(b"\0LOCATE02\0\x00/srv\0\x03Q\0\x03R\0", "/srv\n/srQ\n")]
#[case::past_end_first_entry(b"\0LOCATE02\0\x01/srv\0", "")]
fn test_locate_rejects_out_of_range_prefix(#[case] db: &[u8], #[case] valid_entries: &str) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.db");
    std::fs::write(&path, db).unwrap();
    let assert = Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .args(["/", &format!("--database={}", path.display())])
        .assert()
        .failure();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert_eq!(stdout, valid_entries);
    assert!(stderr.contains("corrupt or invalid"), "{stderr}");
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
fn test_locate_large_max_database_age() {
    for age in ["1000000000000", "18446744073709551615"] {
        Command::cargo_bin("locate")
            .expect("couldn't find locate binary")
            .arg("test_data")
            .arg(format!("--max-database-age={age}"))
            .arg(OLD_DB_FLAG)
            .assert()
            .success();
    }
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
fn test_locate_version() {
    let assert = Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .arg("--version")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.starts_with("locate "),
        "expected stdout to start with 'locate ', got: {stdout:?}"
    );
}

#[test]
fn test_updatedb_version() {
    // Regression test: `updatedb --version` used to print
    // "Error: updatedb X.Y.Z" to stderr and exit non-zero. It must print
    // "updatedb X.Y.Z" to stdout and succeed, like the other utilities.
    let assert = Command::cargo_bin("updatedb")
        .expect("couldn't find updatedb binary")
        .arg("--version")
        .assert()
        .success();

    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        stdout.starts_with("updatedb "),
        "expected stdout to start with 'updatedb ', got: {stdout:?}"
    );
    assert!(
        !stdout.contains("Error") && !stderr.contains("Error"),
        "version output must not contain 'Error': stdout={stdout:?}, stderr={stderr:?}"
    );
    assert!(
        stderr.is_empty(),
        "--version should write nothing to stderr, got: {stderr:?}"
    );
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

#[test]
fn test_locate_one_byte_db() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("db_1byte");
    std::fs::write(&db, b"X").unwrap();
    Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .arg(format!("--database={}", db.display()))
        .arg("foo")
        .assert()
        .code(1);
}

#[cfg(unix)]
#[test]
fn test_locate_non_utf8_argument() {
    // A non-UTF-8 argument must produce a clean error, not a panic in
    // std::env::args.
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    let assert = Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .arg(OsString::from_vec(vec![0xff]))
        .assert()
        .code(1);
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("non-UTF-8"),
        "expected an error naming the invalid argument, got: {stderr:?}"
    );
}

#[cfg(unix)]
#[test]
fn test_updatedb_non_utf8_argument() {
    // A non-UTF-8 argument must produce a clean error, not a panic in
    // std::env::args.
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    let assert = Command::cargo_bin("updatedb")
        .expect("couldn't find updatedb binary")
        .arg(OsString::from_vec(vec![0xff]))
        .assert()
        .code(1);
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("non-UTF-8"),
        "expected an error naming the invalid argument, got: {stderr:?}"
    );
}

#[test]
fn test_updatedb_dbformat_locate02() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.db");
    Command::cargo_bin("updatedb")
        .expect("couldn't find updatedb binary")
        .arg("--dbformat=LOCATE02")
        .arg(format!("--localpaths={}", dir.path().display()))
        .arg(format!("--output={}", out.display()))
        .assert()
        .success();
    assert!(out.is_file(), "expected database file at {}", out.display());
}

#[test]
fn test_updatedb_dbformat_invalid() {
    let assert = Command::cargo_bin("updatedb")
        .expect("couldn't find updatedb binary")
        .arg("--dbformat=foo")
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert_eq!(
        stderr,
        "Error: error: invalid value 'foo' for '--dbformat=<dbformat>'\n  [possible values: LOCATE02]\n\nFor more information, try '--help'.\n\n"
    );
}

#[test]
fn test_default_database_paths() {
    assert_eq!(
        findutils::locate::DEFAULT_DB_PATH,
        findutils::updatedb::DEFAULT_DB_PATH
    );
    #[cfg(not(windows))]
    assert_eq!(
        findutils::locate::DEFAULT_DB_PATH,
        "/usr/local/var/locatedb"
    );
    #[cfg(windows)]
    assert_eq!(
        findutils::locate::DEFAULT_DB_PATH,
        r"C:\ProgramData\locatedb"
    );
}

#[test]
fn test_locate_split_database_paths() {
    let joined_paths =
        std::env::join_paths(["test_data/db/test_data_db", "test_data/db/invalid_db"]).unwrap();
    Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .args([
            "abbbc",
            &format!("--database={}", joined_paths.to_string_lossy()),
        ])
        .assert()
        .failure();
}

#[test]
fn test_locate_limit_spans_databases() {
    // --limit caps the whole output, not each database (GNU)
    let joined_paths =
        std::env::join_paths(["test_data/db/test_data_db", "test_data/db/test_data_db"]).unwrap();
    let db_flag = format!("--database={}", joined_paths.to_string_lossy());

    let assert = Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .args(["test_data", "--count", "--limit=3", &db_flag])
        .assert()
        .success();
    assert_eq!(String::from_utf8_lossy(&assert.get_output().stdout), "3\n");

    let assert = Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .args(["test_data", "--null", "--limit=3", &db_flag])
        .assert()
        .success();
    // --null: one of the test entries has a newline in its name
    assert_eq!(
        String::from_utf8_lossy(&assert.get_output().stdout)
            .split_terminator('\0')
            .count(),
        3
    );
}

#[test]
fn test_locate_limit_statistics_reports_every_database() {
    // --statistics still describes each database once --limit is used up
    let joined_paths =
        std::env::join_paths(["test_data/db/test_data_db", "test_data/db/test_data_db"]).unwrap();
    let db_flag = format!("--database={}", joined_paths.to_string_lossy());

    let assert = Command::cargo_bin("locate")
        .expect("couldn't find locate binary")
        .args(["test_data", "--statistics", "--limit=1", &db_flag])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(
        stdout
            .matches("Database test_data/db/test_data_db is in the")
            .count(),
        2
    );
    // the running match count never goes past the limit
    let matching: Vec<&str> = stdout
        .lines()
        .filter(|l| l.starts_with("Matching Filenames:"))
        .collect();
    assert_eq!(matching, ["Matching Filenames: 1", "Matching Filenames: 1"]);
}
