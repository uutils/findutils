// Copyright 2017 Google Inc.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{stdin, stdout, Read, Write};
use std::path::PathBuf;

fn usage() -> ! {
    println!("Simple command-line app just used for testing -exec flags!");
    std::process::exit(2);
}

enum ExitWith {
    Failure,
    UrgentFailure,
    #[cfg(unix)]
    Signal,
}

#[derive(Default)]
struct Config {
    exit_with: Option<ExitWith>,
    print_stdin: bool,
    no_print_cwd: bool,
    destination_dir: Option<String>,
}

fn open_file(destination_dir: &str) -> File {
    let mut file_number = fs::read_dir(destination_dir)
        .expect("failed to read destination")
        .count();

    loop {
        file_number += 1;
        let mut file_path: PathBuf = PathBuf::from(destination_dir);
        file_path.push(format!("{file_number}.txt"));
        if let Ok(f) = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(file_path)
        {
            return f;
        }
    }
}

fn write_content(mut f: impl Write, config: &Config, args: &[String]) {
    if !config.no_print_cwd {
        writeln!(f, "cwd={}", env::current_dir().unwrap().to_string_lossy())
            .expect("failed to write to file");
    }

    if config.print_stdin {
        let mut s = String::new();
        stdin()
            .read_to_string(&mut s)
            .expect("failed to read from stdin");
        writeln!(f, "stdin={}", s.trim()).expect("failed to write to file");
    }

    writeln!(f, "args=").expect("failed to write to file");

    // first two args are going to be the path to this executable and
    // the destination_dir we want to write to. Don't write either of those
    // as they'll be non-deterministic.
    for arg in &args[2..] {
        writeln!(f, "{arg}").expect("failed to write to file");
    }
}

fn main() {
    let args = env::args().collect::<Vec<String>>();
    if args.len() < 2 || args[1] == "-h" || args[1] == "--help" {
        usage();
    }
    let mut config = Config {
        destination_dir: if args[1] != "-" {
            Some(args[1].clone())
        } else {
            None
        },
        ..Default::default()
    };
    for arg in &args[2..] {
        if arg.starts_with("--") {
            match arg.as_ref() {
                "--exit_with_failure" => {
                    config.exit_with = Some(ExitWith::Failure);
                }
                "--exit_with_urgent_failure" => {
                    config.exit_with = Some(ExitWith::UrgentFailure);
                }
                #[cfg(unix)]
                "--exit_with_signal" => {
                    config.exit_with = Some(ExitWith::Signal);
                }
                "--no_print_cwd" => {
                    config.no_print_cwd = true;
                }
                "--print_stdin" => {
                    config.print_stdin = true;
                }
                _ => {
                    usage();
                }
            }
        }
    }

    if let Some(destination_dir) = &config.destination_dir {
        write_content(open_file(destination_dir), &config, &args);
    } else {
        write_content(stdout(), &config, &args);
    }

    match config.exit_with {
        None => std::process::exit(0),
        Some(ExitWith::Failure) => std::process::exit(2),
        Some(ExitWith::UrgentFailure) => std::process::exit(255),
        #[cfg(unix)]
        Some(ExitWith::Signal) => unsafe {
            uucore::libc::raise(uucore::libc::SIGINT);
        },
    }
}
