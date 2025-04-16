// Copyright 2017 Google Inc.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

#[cfg(test)]
mod tests {
    use std::process::Command;

    use assert_cmd::{assert::OutputAssertExt, cargo::CommandCargoExt};

    #[test]
    fn test_locate_no_matches() {
        Command::cargo_bin("locate")
            .expect("couldn't find locate binary")
            .args(["usr", "--database=test_data_db"])
            .assert()
            .failure();
    }

    #[test]
    fn test_locate_match() {
        Command::cargo_bin("locate")
            .expect("couldn't find locate binary")
            .args(["test_data", "--database=test_data_db"])
            .assert()
            .success();
    }

    #[test]
    fn test_locate_no_matches_basename() {
        Command::cargo_bin("locate")
            .expect("couldn't find locate binary")
            .args([
                "test_data1234567890",
                "--basename",
                "--database=test_data_db",
            ])
            .assert()
            .failure();
    }

    #[test]
    fn test_locate_match_basename() {
        Command::cargo_bin("locate")
            .expect("couldn't find locate binary")
            .args(["abbbc", "--basename", "--database=test_data_db"])
            .assert()
            .success();
    }

    #[test]
    fn test_locate_existing() {
        Command::cargo_bin("locate")
            .expect("couldn't find locate binary")
            .args(["abbbc", "--existing", "--database=test_data_db"])
            .assert()
            .success();
    }

    #[test]
    fn test_locate_non_existing() {
        Command::cargo_bin("locate")
            .expect("couldn't find locate binary")
            .args(["abbbc", "--non-existing", "--database=test_data_db"])
            .assert()
            .failure();
    }

    #[test]
    fn test_locate_statistics() {
        Command::cargo_bin("locate")
            .expect("couldn't find locate binary")
            .args(["abbbc", "--statistics", "--database=test_data_db"])
            .assert()
            .success();
    }

    #[test]
    fn test_locate_regex() {
        Command::cargo_bin("locate")
            .expect("couldn't find locate binary")
            .args(["abbbc", "--regex", "--database=test_data_db"])
            .assert()
            .success();
    }

    #[test]
    fn test_locate_all() {
        Command::cargo_bin("locate")
            .expect("couldn't find locate binary")
            .args(["abb", "bbc", "--regex", "--database=test_data_db"])
            .assert()
            .success();
    }

    #[test]
    fn test_locate_all_regex() {
        Command::cargo_bin("locate")
            .expect("couldn't find locate binary")
            .args(["abb", "b*c", "--regex", "--database=test_data_db"])
            .assert()
            .success();
    }

    #[test]
    fn test_updatedb() {
        Command::cargo_bin("updatedb")
            .expect("couldn't find updatedb binary")
            .args(["--localpaths=./test_data", "--output=/dev/null"])
            .assert()
            .success();
    }
}
