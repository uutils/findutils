use std::{
    default, env,
    error::Error,
    ffi::{CString, OsStr, OsString},
    fmt::Display,
    fs::File,
    io::{stderr, BufRead, BufReader, Read, Write},
    os::unix::ffi::OsStringExt,
    path::{Path, PathBuf},
};

use clap::{crate_version, value_parser, Arg, ArgAction, ArgMatches, Command};
use regex::Regex;
use uucore::error::{UClapError, UError, UResult};

pub struct Config {
    all: bool,
    basename: bool,
    mode: Mode,
    db: PathBuf,
    existing: ExistenceMode,
    follow_symlinks: bool,
    ignore_case: bool,
    limit: Option<usize>,
    max_age: usize,
    null_bytes: bool,
    print: bool,
    regex: Option<RegexType>,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Mode {
    #[default]
    Normal,
    Count,
    Statistics,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum ExistenceMode {
    #[default]
    Any,
    Present,
    NotPresent,
}

#[derive(Debug, Clone, Copy)]
pub enum RegexType {
    FindutilsDefault,
    Emacs,
    GnuAwk,
    Grep,
    PosixAwk,
    Awk,
    PosixBasic,
    PosixEgrep,
    Egrep,
    PosixExtended,
}

#[derive(Debug)]
pub enum LocateError {
    InvalidDbType,
}

impl Display for LocateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LocateError::InvalidDbType => f.write_str("Unknown database type"),
        }
    }
}

impl Error for LocateError {}

impl UError for LocateError {
    fn code(&self) -> i32 {
        1
    }

    fn usage(&self) -> bool {
        match self {
            LocateError::InvalidDbType => false,
        }
    }
}

pub struct ParsedInfo {
    patterns: Vec<String>,
    config: Config,
}

impl From<ArgMatches> for ParsedInfo {
    fn from(value: ArgMatches) -> Self {
        Self {
            patterns: value
                .get_many::<String>("patterns")
                .unwrap()
                .cloned()
                .collect(),
            config: Config {
                all: value.get_flag("all"),
                basename: value.get_flag("basename"),
                db: value.get_one::<PathBuf>("database").cloned().unwrap(),
                mode: value
                    .get_many::<String>("mode")
                    .unwrap_or_default()
                    .last()
                    .map(|s| match s.as_str() {
                        "count" => Mode::Count,
                        "statistics" => Mode::Statistics,
                        _ => unreachable!(),
                    })
                    .unwrap_or_default(),
                existing: value
                    .get_many::<String>("exist")
                    .unwrap_or_default()
                    .last()
                    .map(|s| match s.as_str() {
                        "existing" => ExistenceMode::Present,
                        "statistics" => ExistenceMode::NotPresent,
                        _ => unreachable!(),
                    })
                    .unwrap_or_default(),
                follow_symlinks: value.get_flag("follow") || !value.get_flag("nofollow"),
                ignore_case: value.get_flag("ignore-case"),
                limit: value.get_one::<usize>("limit").copied(),
                max_age: *value.get_one::<usize>("max-database-age").unwrap(),
                null_bytes: value.get_flag("null"),
                print: value.get_flag("print"),
                regex: value
                    .get_flag("regex")
                    .then(|| value.get_one::<RegexType>("regextype"))
                    .flatten()
                    .cloned(),
            },
        }
    }
}

fn uu_app() -> Command {
    Command::new("locate")
        .version(crate_version!())
        .args_override_self(true)
        .arg(
            Arg::new("all")
                .short('a')
                .long("all")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("basename")
                .short('b')
                .long("basename")
                .action(ArgAction::SetTrue)
                .group("name"),
        )
        .arg(
            Arg::new("count")
                .short('c')
                .long("count")
                .action(ArgAction::SetTrue)
                .group("mode"),
        )
        .arg(
            Arg::new("database")
                .short('d')
                .long("database")
                .env("LOCATE_PATH")
                .default_value("/usr/local/var/locatedb")
                .value_parser(value_parser!(PathBuf))
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("existing")
                .short('e')
                .long("existing")
                .action(ArgAction::SetTrue)
                .group("exist"),
        )
        .arg(
            Arg::new("non-existing")
                .short('E')
                .long("non-existing")
                .action(ArgAction::SetTrue)
                .group("exist"),
        )
        .arg(
            Arg::new("follow")
                .short('L')
                .action(ArgAction::SetTrue)
                .overrides_with("nofollow"),
        )
        .arg(
            Arg::new("nofollow")
                .short('P')
                .short_alias('H')
                .action(ArgAction::SetTrue)
                .overrides_with("follow"),
        )
        .arg(
            Arg::new("ignore-case")
                .short('i')
                .long("ignore-case")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("limit")
                .short('l')
                .long("limit")
                .value_parser(value_parser!(usize))
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("max-database-age")
                .long("max-database-age")
                .value_parser(value_parser!(usize))
                .default_value("8")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("mmap")
                .short('m')
                .long("mmap")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("null")
                .short('0')
                .long("null")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("print")
                .short('p')
                .long("print")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("wholename")
                .short('w')
                .long("wholename")
                .action(ArgAction::SetFalse)
                .group("name"),
        )
        .arg(
            Arg::new("regex")
                .short('r')
                .long("regex")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("regextype")
                .long("regextype")
                .value_parser([
                    "findutils-default",
                    "emacs",
                    "gnu-awk",
                    "grep",
                    "posix-awk",
                    "awk",
                    "posix-basic",
                    "posix-egrep",
                    "egrep",
                    "posix-extended",
                ])
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("stdio")
                .short('s')
                .long("stdio")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("statistics")
                .short('S')
                .long("statistics")
                .action(ArgAction::SetTrue)
                .group("mode"),
        )
        .arg(
            Arg::new("patterns")
                .num_args(1..)
                .action(ArgAction::Append)
                .value_parser(value_parser!(String))
                .required(true),
        )
}

struct DbReader {
    reader: BufReader<File>,
    prev: Option<CString>,
    prefix: usize,
}

impl Iterator for DbReader {
    type Item = CString;

    fn next(&mut self) -> Option<Self::Item> {
        let mut buf = [0];
        self.reader.read_exact(&mut buf).ok()?;
        let size = if buf[0] == 0x80 {
            let mut buf = [0; 2];
            self.reader.read_exact(&mut buf).ok()?;
            u16::from_be_bytes(buf) as isize
        } else {
            buf[0] as isize
        };
        self.prefix = (self.prefix as isize + size) as usize;
        let mut buf = Vec::new();
        self.reader.read_until(b'\0', &mut buf).ok()?;
        let prefix = self
            .prev
            .as_ref()
            .map(|s| s.to_bytes().iter().take(self.prefix).collect::<Vec<_>>());
        if (prefix.as_ref().map(|v| v.len()).unwrap_or(0) as isize) < size {
            return None;
        }
        let res = CString::from_vec_with_nul(
            prefix
                .unwrap_or_else(|| vec![])
                .into_iter()
                .copied()
                .chain(buf)
                .collect(),
        )
        .ok()?;
        self.prev = Some(res.clone());
        Some(res)
    }
}

impl DbReader {
    fn new(path: impl AsRef<Path>) -> UResult<Self> {
        let mut s = Self {
            reader: BufReader::new(File::open(path.as_ref())?),
            prev: None,
            prefix: 0,
        };
        match s.check_db() {
            true => Ok(s),
            false => Err(Box::new(LocateError::InvalidDbType)),
        }
    }

    fn check_db(&mut self) -> bool {
        let mut buf = [0];
        let Ok(_) = self.reader.read_exact(&mut buf) else {
            return false;
        };
        let mut buf = Vec::new();
        let Ok(_) = self.reader.read_until(b'\0', &mut buf) else {
            return false;
        };
        match String::from_utf8_lossy(buf.as_slice()).as_ref() {
            "LOCATE02" => true,
            _ => false,
        }
    }
}

fn match_entry(entry: &OsStr, config: &Config, patterns: &[String]) -> bool {
    let buf = PathBuf::from(entry);
    let entry = if config.basename {
        let Some(path) = buf.file_name() else {
            return false;
        };

        path
    } else {
        entry
    };
    match config.regex {
        Some(_) => patterns
            .iter()
            .filter_map(|s| Regex::new(s).ok())
            .any(|r| r.is_match(entry.to_string_lossy().as_ref())),
        None => {
            if entry
                .to_string_lossy()
                .chars()
                .any(|c| r"*?[]\".contains(c))
            {
                // parse metacharacters
                false
            } else {
                patterns
                    .iter()
                    .any(|s| s.contains(entry.to_string_lossy().as_ref()))
            }
        }
    }
}

fn do_locate(args: &[&str]) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args);
    match matches {
        Err(e) => {
            let mut app = uu_app();

            match e.kind() {
                clap::error::ErrorKind::DisplayHelp => {
                    app.print_help()?;
                }
                clap::error::ErrorKind::DisplayVersion => print!("{}", app.render_version()),
                _ => return Err(Box::new(e.with_exit_code(1))),
            }
        }
        Ok(matches) => {
            let ParsedInfo { patterns, config } = ParsedInfo::from(matches);
            let dbreader = DbReader::new(config.db.as_path())?;

            for s in dbreader {
                if match_entry(s.as_os_str(), &config, patterns.as_slice()) {
                    println!("{}", s.to_string_lossy());
                }
            }
        }
    }

    Ok(())
}

pub fn locate_main(args: &[&str]) -> i32 {
    match do_locate(&args[1..]) {
        Ok(()) => 0,
        Err(e) => {
            writeln!(&mut stderr(), "Error: {e}").unwrap();
            1
        }
    }
}
