// Copyright 2017 Google Inc.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

fn usage() -> ! {
    println!("Simple command-line app just used for testing -exec flags!");
    std::process::exit(2);
}

#[derive(Default)]
struct Config {
    exit_with_failure: bool,
    destination_dir: String,
}

fn open_file(destination_dir: &str) -> File {
    let mut file_number =
        fs::read_dir(destination_dir).expect("failed to read destination").count();

    loop {
        file_number += 1;
        let mut file_path: PathBuf = PathBuf::from(destination_dir);
        file_path.push(format!("{}.txt", file_number));
        if let Ok(f) = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(file_path) {
            return f;
        }
    }
}

fn main() {
    let args = env::args().collect::<Vec<String>>();
    if args.len() < 2 || args[1] == "-h" || args[1] == "--help" {
        usage();
    }
    let mut config = Config::default();
    config.destination_dir = args[1].clone();
    for arg in &args[2..] {
        if arg.starts_with("--") {
            match arg.as_ref() {
                "--exit_with_failure" => {
                    config.exit_with_failure = true;
                }
                _ => {
                    usage();
                }
            }
        }
    }

    {
        let mut f = open_file(&config.destination_dir);
        // first two args are going to be the path to this executable and
        // the destination_dir we want to write to. Don't write either of those
        // as they'll be non-deterministic.
        f.write_fmt(format_args!("cwd={}\nargs=\n",
                                    env::current_dir().unwrap().to_string_lossy()))
            .expect("failed to write to file");
        for arg in &args[2..] {
            f.write_fmt(format_args!("{}\n", arg)).expect("failed to write to file");
        }

    }
    std::process::exit(if config.exit_with_failure { 2 } else { 0 });
}
