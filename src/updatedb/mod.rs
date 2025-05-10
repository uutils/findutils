// Copyright 2017 Google Inc.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::{
    cell::RefCell,
    fmt::Display,
    fs::OpenOptions,
    io::{stderr, BufRead, BufReader, BufWriter, Write},
    path::PathBuf,
    rc::Rc,
    str::FromStr,
    time::SystemTime,
};

use clap::{crate_version, value_parser, Arg, ArgAction, ArgMatches, Command};
use itertools::Itertools;
use uucore::error::UResult;

use crate::find::{find_main, Dependencies};

// local_user and net_user are currently ignored
#[allow(dead_code)]
pub struct Config {
    find_options: String,
    local_paths: Vec<PathBuf>,
    net_paths: Vec<String>,
    prune_paths: Vec<PathBuf>,
    prune_fs: Vec<String>,
    output: PathBuf,
    local_user: Option<String>,
    net_user: String,
    db_format: DbFormat,
}

impl From<ArgMatches> for Config {
    fn from(value: ArgMatches) -> Self {
        Self {
            find_options: value
                .get_one::<String>("findoptions")
                .cloned()
                .unwrap_or_else(String::new),
            local_paths: value
                .get_one::<String>("localpaths")
                .map(|s| {
                    s.split_whitespace()
                        .filter_map(|s| PathBuf::from_str(s).ok())
                        .collect()
                })
                .unwrap_or_else(|| vec![PathBuf::from("/")]),
            net_paths: value
                .get_one::<String>("netpaths")
                .map(|s| s.split_whitespace().map(|s| s.to_owned()).collect())
                .unwrap_or_default(),
            prune_paths: value
                .get_one::<String>("prunepaths")
                .map(|s| s.split_whitespace().map(PathBuf::from).collect())
                .unwrap_or_else(|| {
                    ["/tmp", "/usr/tmp", "/var/tmp", "/afs"]
                        .into_iter()
                        .map(PathBuf::from)
                        .collect()
                }),
            prune_fs: value
                .get_one::<String>("prunefs")
                .map(|s| s.split_whitespace().map(|s| s.to_owned()).collect())
                .unwrap_or_else(|| {
                    ["nfs", "NFS", "proc"]
                        .into_iter()
                        .map(str::to_string)
                        .collect()
                }),
            db_format: value
                .get_one::<DbFormat>("dbformat")
                .copied()
                .unwrap_or_default(),
            output: value
                .get_one::<PathBuf>("output")
                .cloned()
                // FIXME: the default should be platform-dependent
                .unwrap_or(PathBuf::from_str("/usr/local/var/locatedb").unwrap()),
            local_user: value.get_one::<String>("localuser").cloned(),
            net_user: value
                .get_one::<String>("netuser")
                .cloned()
                .unwrap_or(String::from("daemon")),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum DbFormat {
    #[default]
    Locate02,
}

// used for locate's --statistics
impl Display for DbFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Locate02 => f.write_str("GNU LOCATE02"),
        }
    }
}

fn uu_app() -> Command {
    Command::new("updatedb")
        .version(crate_version!())
        .arg(
            Arg::new("findoptions")
                .long("findoptions")
                .require_equals(true)
                .env("FINDOPTIONS")
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("localpaths")
                .long("localpaths")
                .require_equals(true)
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("netpaths")
                .long("netpaths")
                .require_equals(true)
                .env("NETPATHS")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("prunepaths")
                .long("prunepaths")
                .require_equals(true)
                .env("PRUNEPATHS")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("prunefs")
                .long("prunefs")
                .require_equals(true)
                .env("PRUNEFS")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .require_equals(true)
                .value_parser(value_parser!(PathBuf))
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("localuser")
                .long("localuser")
                .require_equals(true)
                .env("LOCALUSER")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("netuser")
                .long("netuser")
                .require_equals(true)
                .env("NETUSER")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("dbformat")
                .long("dbformat")
                .require_equals(true)
                .value_parser(["LOCATE02"])
                .action(ArgAction::Set),
        )
}

// The LOCATE02 format elides bytes from the path until the first byte that differs from the
// previous entry. It keeps a running total of the prefix length, and uses 1 or 3 bytes to write
// the difference from the previous prefix length. Paths are provided in sorted order by find.
struct Frcoder<'a> {
    reader: BufReader<&'a [u8]>,
    prev: Option<Vec<u8>>,
    prefix: usize,
    ty: DbFormat,
}

impl<'a> Frcoder<'a> {
    fn new(v: &'a [u8], ty: DbFormat) -> Self {
        Self {
            reader: BufReader::new(v),
            prev: None,
            prefix: 0,
            ty,
        }
    }

    fn generate_header(&self) -> Vec<u8> {
        match self.ty {
            DbFormat::Locate02 => "\0LOCATE02\0".as_bytes().to_vec(),
        }
    }
}

impl Iterator for Frcoder<'_> {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut path = Vec::new();
        // find prints nul bytes after each path
        if self.reader.read_until(b'\0', &mut path).ok()? == 0 {
            return None;
        }

        let prefix = path
            .iter()
            .zip(self.prev.as_deref().unwrap_or_default())
            .take_while(|(a, b)| a == b)
            .count();

        let diff = prefix as i32 - self.prefix as i32;

        // if the prefix delta exceeds 0x7f, we use 0x80 to signal that the next two bytes comprise
        // the delta
        let mut out = Vec::new();
        if diff.abs() > 0x7f {
            out.push(0x80);
            out.extend((diff as i16).to_be_bytes());
        } else {
            out.push(diff as u8);
        }

        out.extend(path.iter().skip(prefix));

        self.prefix = prefix;
        self.prev = Some(path);

        Some(out)
    }
}

// capture find's stdout
struct CapturedDependencies {
    output: Rc<RefCell<dyn Write>>,
    now: SystemTime,
}

impl CapturedDependencies {
    fn new(output: Rc<RefCell<dyn Write>>) -> Self {
        Self {
            output,
            now: SystemTime::now(),
        }
    }
}

impl Dependencies for CapturedDependencies {
    fn get_output(&self) -> &RefCell<dyn Write> {
        self.output.as_ref()
    }

    fn now(&self) -> SystemTime {
        self.now
    }
}

fn do_updatedb(args: &[&str]) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;
    let config = Config::from(matches);

    // TODO: handle localuser and netuser
    // this will likely involve splitting the find logic into two calls

    let mut find_args = vec!["find"];
    find_args.extend(config.local_paths.iter().filter_map(|p| p.to_str()));
    find_args.extend(config.net_paths.iter().map(|s| s.as_str()));
    find_args.extend(config.find_options.split_whitespace());
    // offload most of the logic to find
    let excludes = format!(
        "( {} {} ) -prune {} {} {}",
        if config.prune_fs.is_empty() {
            ""
        } else {
            "-fstype"
        },
        config.prune_fs.iter().join(" -or -fstype "),
        if config.prune_paths.is_empty() {
            ""
        } else {
            "-or -regex"
        },
        config
            .prune_paths
            .iter()
            .filter_map(|p| p.to_str())
            .join(" -prune -or -regex "),
        if config.prune_paths.is_empty() {
            ""
        } else {
            "-prune"
        },
    );
    find_args.extend(excludes.split_whitespace());
    find_args.extend(["-or", "-print0", "-sorted"]);

    let output = Rc::new(RefCell::new(Vec::new()));
    let deps = CapturedDependencies::new(output.clone());
    find_main(find_args.as_slice(), &deps);

    let mut writer = BufWriter::new(
        OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(config.output)?,
    );

    let output = output.borrow();
    let frcoder = Frcoder::new(output.as_slice(), config.db_format);
    writer.write_all(&frcoder.generate_header())?;
    for v in frcoder {
        writer.write_all(v.as_slice())?;
    }

    writer.flush()?;

    Ok(())
}

pub fn updatedb_main(args: &[&str]) -> i32 {
    match do_updatedb(args) {
        Ok(()) => 0,
        Err(e) => {
            writeln!(&mut stderr(), "Error: {e}").unwrap();
            1
        }
    }
}
