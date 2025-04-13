// Copyright 2017 Google Inc.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/// ! This file contains what would be normally be unit tests for `find::find_main`
/// ! related to -exec[dir] and ok[dir] clauses.
/// ! But as the tests require running an external executable, they need to be run
/// ! as integration tests so we can ensure that our testing-commandline binary
/// ! has been built.
use std::env;
use std::fs::File;
use std::io::Read;
use tempfile::Builder;

use common::test_helpers::{fix_up_slashes, path_to_testing_commandline, FakeDependencies};
use findutils::find::find_main;

mod common;
#[test]
fn find_exec() {
    let temp_dir = tempfile::Builder::new()
        .prefix("find_exec")
        .tempdir()
        .unwrap();
    let temp_dir_path = temp_dir.path().to_string_lossy();
    let deps = FakeDependencies::new();

    let rc = find_main(
        &[
            "find",
            &fix_up_slashes("./test_data/simple/subdir"),
            "-type",
            "f",
            "-exec",
            &path_to_testing_commandline(),
            temp_dir_path.as_ref(),
            "(",
            "{}",
            "-o",
            ";",
        ],
        &deps,
    );

    assert_eq!(rc, 0);
    // exec has side effects, so we won't output anything unless -print is
    // explicitly passed in.
    assert_eq!(deps.get_output_as_string(), "");

    // check the executable ran as expected
    let mut f = File::open(temp_dir.path().join("1.txt")).expect("Failed to open output file");
    let mut s = String::new();
    f.read_to_string(&mut s)
        .expect("failed to read output file");
    assert_eq!(
        s,
        fix_up_slashes(&format!(
            "cwd={}\nargs=\n(\n./test_data/simple/subdir/ABBBC\n-o\n",
            env::current_dir().unwrap().to_string_lossy()
        ))
    );
}

#[test]
fn find_execdir() {
    let temp_dir = Builder::new().prefix("example").tempdir().unwrap();
    let temp_dir_path = temp_dir.path().to_string_lossy();
    let deps = FakeDependencies::new();
    // only look at files because the "size" of a directory is a system (and filesystem)
    // dependent thing and we want these tests to be universal.
    let rc = find_main(
        &[
            "find",
            &fix_up_slashes("./test_data/simple/subdir"),
            "-type",
            "f",
            "-execdir",
            &path_to_testing_commandline(),
            temp_dir_path.as_ref(),
            ")",
            "{}",
            ",",
            ";",
        ],
        &deps,
    );

    assert_eq!(rc, 0);
    // exec has side effects, so we won't output anything unless -print is
    // explicitly passed in.
    assert_eq!(deps.get_output_as_string(), "");

    // check the executable ran as expected
    let mut f = File::open(temp_dir.path().join("1.txt")).expect("Failed to open output file");
    let mut s = String::new();
    f.read_to_string(&mut s)
        .expect("failed to read output file");
    assert_eq!(
        s,
        fix_up_slashes(&format!(
            "cwd={}/test_data/simple/subdir\nargs=\n)\n./ABBBC\n,\n",
            env::current_dir().unwrap().to_string_lossy()
        ))
    );
}

#[test]
fn find_exec_multi() {
    let temp_dir = tempfile::Builder::new()
        .prefix("find_exec_multi")
        .tempdir()
        .unwrap();
    let temp_dir_path = temp_dir.path().to_string_lossy();
    let deps = FakeDependencies::new();

    let rc = find_main(
        &[
            "find",
            &fix_up_slashes("./test_data/simple"),
            "-type",
            "f",
            "-exec",
            &path_to_testing_commandline(),
            temp_dir_path.as_ref(),
            "--sort",
            "(",
            "-o",
            "{}",
            "+",
        ],
        &deps,
    );

    assert_eq!(rc, 0);
    // exec has side effects, so we won't output anything unless -print is
    // explicitly passed in.
    assert_eq!(deps.get_output_as_string(), "");

    // check the executable ran as expected
    let mut f = File::open(temp_dir.path().join("1.txt")).expect("Failed to open output file");
    let mut s = String::new();
    f.read_to_string(&mut s)
        .expect("failed to read output file");
    assert_eq!(
        s,
        fix_up_slashes(&format!(
            "cwd={}\nargs=\n(\n--sort\n-o\n./test_data/simple/abbbc\n./test_data/simple/subdir/ABBBC\n",
            env::current_dir().unwrap().to_string_lossy()
        ))
    );
}

#[test]
fn find_execdir_multi() {
    let temp_dir = Builder::new()
        .prefix("find_execdir_multi")
        .tempdir()
        .unwrap();
    let temp_dir_path = temp_dir.path().to_string_lossy();
    let deps = FakeDependencies::new();
    // only look at files because the "size" of a directory is a system (and filesystem)
    // dependent thing and we want these tests to be universal.
    let rc = find_main(
        &[
            "find",
            &fix_up_slashes("./test_data/simple"),
            "-maxdepth",
            "1",
            "-execdir",
            &path_to_testing_commandline(),
            temp_dir_path.as_ref(),
            "--sort",
            ")",
            "{}",
            "+",
        ],
        &deps,
    );

    assert_eq!(rc, 0);
    // exec has side effects, so we won't output anything unless -print is
    // explicitly passed in.
    assert_eq!(deps.get_output_as_string(), "");

    // check the executable ran as expected
    let mut f = File::open(temp_dir.path().join("1.txt")).expect("Failed to open output file");
    let mut s = String::new();
    f.read_to_string(&mut s)
        .expect("failed to read output file");
    assert_eq!(
        s,
        fix_up_slashes(&format!(
            "cwd={}/test_data\nargs=\n)\n--sort\n./simple\n",
            env::current_dir().unwrap().to_string_lossy()
        ))
    );
    let mut f = File::open(temp_dir.path().join("2.txt")).expect("Failed to open output file");
    let mut s = String::new();
    f.read_to_string(&mut s)
        .expect("failed to read output file");
    assert_eq!(
        s,
        fix_up_slashes(&format!(
            "cwd={}/test_data/simple\nargs=\n)\n--sort\n./abbbc\n./subdir\n",
            env::current_dir().unwrap().to_string_lossy()
        ))
    );
}

#[test]
fn find_execdir_multi_in_root_directory() {
    let temp_dir = Builder::new()
        .prefix("find_execdir_multi_in_root_directory")
        .tempdir()
        .unwrap();
    let temp_dir_path = temp_dir.path().to_string_lossy();
    let deps = FakeDependencies::new();

    let cwd = env::current_dir().expect("no current directory");
    let root_dir = cwd
        .ancestors()
        .last()
        .expect("current directory has no root");

    // only look at files because the "size" of a directory is a system (and filesystem)
    // dependent thing and we want these tests to be universal.
    let rc = find_main(
        &[
            "find",
            &fix_up_slashes(&root_dir.to_string_lossy()),
            "-maxdepth",
            "0",
            "-execdir",
            &path_to_testing_commandline(),
            temp_dir_path.as_ref(),
            "--sort",
            ")",
            "{}",
            "+",
        ],
        &deps,
    );

    assert_eq!(rc, 0);
    // exec has side effects, so we won't output anything unless -print is
    // explicitly passed in.
    assert_eq!(deps.get_output_as_string(), "");

    // check the executable ran as expected
    let mut f = File::open(temp_dir.path().join("1.txt")).expect("Failed to open output file");
    let mut s = String::new();
    f.read_to_string(&mut s)
        .expect("failed to read output file");
    assert_eq!(
        s,
        fix_up_slashes(&format!(
            "cwd={}\nargs=\n)\n--sort\n{}\n",
            root_dir.to_string_lossy(),
            root_dir.to_string_lossy(),
        ))
    );
}
