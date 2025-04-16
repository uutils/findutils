// Copyright 2017 Google Inc.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::{
    borrow::Cow,
    env,
    ffi::{CStr, CString, OsStr},
    fs::{self, File},
    io::{self, stderr, BufRead, BufReader, Read, Write},
    os::unix::{ffi::OsStrExt, fs::MetadataExt},
    path::{Path, PathBuf},
    str::FromStr,
};

use chrono::{DateTime, Local, TimeDelta};
use clap::{self, crate_version, value_parser, Arg, ArgAction, ArgMatches, Command, Id};
use onig::{Regex, RegexOptions, Syntax};
use quick_error::quick_error;
use uucore::error::{ClapErrorWrapper, UClapError, UError, UResult};

use crate::{find::matchers::RegexType, updatedb::DbFormat};

#[derive(Debug)]
pub struct Config {
    all: bool,
    basename: bool,
    mode: Mode,
    db: Vec<PathBuf>,
    existing: ExistenceMode,
    follow_symlinks: bool,
    ignore_case: bool,
    limit: Option<usize>,
    max_age: usize,
    null_bytes: bool,
    print: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
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

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        NoMatches {}
        InvalidDbType { display("Unknown database type") }
        IoErr(err: io::Error) { from() source(err) display("{err}") }
        ClapErr(err: ClapErrorWrapper) { from() source(err) display("{err}") }
        /// General copy error
        Error(err: String) {
            display("{err}")
            from(err: String) -> (err)
            from(err: &'static str) -> (err.to_string())
        }

    }
}

type LocateResult<T> = Result<T, Error>;

impl UError for Error {
    fn code(&self) -> i32 {
        1
    }
}

pub struct Statistics {
    matches: usize,
    total_length: usize,
    whitespace: usize,
    newlines: usize,
    high_bit: usize,
}

impl Statistics {
    fn new() -> Self {
        Self {
            matches: 0,
            total_length: 0,
            whitespace: 0,
            newlines: 0,
            high_bit: 0,
        }
    }

    fn add_match(&mut self, mat: &CStr) {
        let s = mat.to_string_lossy();
        self.matches += 1;
        self.total_length += s.len();
        if s.chars().any(char::is_whitespace) {
            self.whitespace += 1;
        }
        if s.chars().any(|c| c == '\n') {
            self.newlines += 1;
        }
        if !s.is_ascii() {
            self.high_bit += 1;
        }
    }

    fn print_header(&self, dbreader: &DbReader) {
        println!(
            "Database {} is in the {} format.",
            dbreader.path.to_string_lossy(),
            dbreader.format,
        );
    }

    fn print(&self, dbreader: &DbReader) {
        if let Ok(metadata) = fs::metadata(&dbreader.path) {
            if let Ok(time) = metadata.modified() {
                let time: DateTime<Local> = time.into();
                println!("Database was last modified at {}", time);
            }
            println!("Locate database size: {} bytes", metadata.size());
        }
        println!("Matching Filenames: {}", self.matches);
        println!(
            "File names have a cumulative length of {} bytes",
            self.total_length
        );
        println!("Of those file names,\n");
        println!("        {} contain whitespace,", self.whitespace);
        println!("        {} contain newline characters,", self.newlines);
        println!(
            "        and {} contain characters with the high bit set.",
            self.high_bit
        );
        println!();
    }
}

enum Patterns {
    String(Vec<String>),
    Regex(Vec<Regex>),
}

impl Patterns {
    fn any_match(&self, entry: &str) -> bool {
        match self {
            Self::String(v) => v.iter().any(|s| entry.contains(s)),
            Self::Regex(v) => v.iter().any(|r| r.is_match(entry)),
        }
    }

    fn all_match(&self, entry: &str) -> bool {
        match self {
            Self::String(v) => v.iter().all(|s| entry.contains(s)),
            Self::Regex(v) => v.iter().all(|r| r.is_match(entry)),
        }
    }
}

pub struct ParsedInfo {
    patterns: Patterns,
    config: Config,
}

fn make_regex(ty: RegexType, config: &Config, pattern: &str) -> Option<Regex> {
    let syntax = match ty {
        RegexType::Emacs => Syntax::emacs(),
        RegexType::Grep => Syntax::grep(),
        RegexType::PosixBasic => Syntax::posix_basic(),
        RegexType::PosixExtended => Syntax::posix_extended(),
    };

    Regex::with_options(
        pattern,
        if config.ignore_case {
            RegexOptions::REGEX_OPTION_IGNORECASE
        } else {
            RegexOptions::REGEX_OPTION_NONE
        },
        syntax,
    )
    .ok()
}

impl From<ArgMatches> for ParsedInfo {
    fn from(value: ArgMatches) -> Self {
        let config = Config {
            all: value.get_flag("all"),
            basename: value.get_flag("basename"),
            db: value
                .get_one::<String>("database")
                .unwrap()
                .split(':')
                .map(PathBuf::from)
                .collect(),
            mode: value
                .get_many::<Id>("mode")
                .unwrap_or_default()
                .next_back()
                .map(|s| match s.as_str() {
                    "count" => Mode::Count,
                    "statistics" => Mode::Statistics,
                    _ => unreachable!(),
                })
                .unwrap_or_default(),
            existing: value
                .get_many::<Id>("exist")
                .unwrap_or_default()
                .next_back()
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
        };
        let patterns: Vec<String> = value
            .get_many::<String>("patterns")
            .unwrap()
            .cloned()
            .collect();
        let patterns = if let Some(ty) = value
            .get_flag("regex")
            .then(|| {
                value
                    .get_one::<String>("regextype")
                    .and_then(|s| RegexType::from_str(s.as_str()).ok())
            })
            .flatten()
        {
            Patterns::Regex(
                patterns
                    .into_iter()
                    .filter_map(|s| make_regex(ty, &config, &s))
                    .collect(),
            )
        } else {
            Patterns::String(patterns)
        };
        Self { patterns, config }
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
    prefix: isize,
    format: DbFormat,
    path: PathBuf,
}

impl Iterator for DbReader {
    type Item = CString;

    fn next(&mut self) -> Option<Self::Item> {
        // 1 byte for the prefix delta
        let mut buf = [0];
        self.reader.read_exact(&mut buf).ok()?;
        // 0x80 - the prefix delta takes the next two bytes
        let size = if buf[0] == 0x80 {
            let mut buf = [0; 2];
            self.reader.read_exact(&mut buf).ok()?;
            i16::from_be_bytes(buf) as isize
        } else {
            // u8 as isize directly doesn't sign-extend
            buf[0] as i8 as isize
        };
        self.prefix += size;
        // read the actual path fragment
        let mut buf = Vec::new();
        self.reader.read_until(b'\0', &mut buf).ok()?;
        let prefix = self.prev.as_ref().map(|s| {
            s.to_bytes()
                .iter()
                .take(self.prefix as usize)
                .collect::<Vec<_>>()
        });
        if (prefix.as_ref().map(|v| v.len()).unwrap_or(0) as isize) < size {
            return None;
        }
        let res = CString::from_vec_with_nul(
            prefix
                .unwrap_or_default()
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
        let mut reader = BufReader::new(File::open(path.as_ref())?);
        let format = Self::check_db(&mut reader).ok_or(Error::InvalidDbType)?;
        Ok(Self {
            reader,
            prev: None,
            prefix: 0,
            format,
            path: path.as_ref().to_path_buf(),
        })
    }

    fn check_db(reader: &mut BufReader<File>) -> Option<DbFormat> {
        let mut buf = [0];
        let Ok(_) = reader.read_exact(&mut buf) else {
            return None;
        };
        let mut buf = Vec::new();
        let Ok(_) = reader.read_until(b'\0', &mut buf) else {
            return None;
        };

        // drop nul byte when matching
        match String::from_utf8_lossy(&buf[..buf.len() - 1]).as_ref() {
            "LOCATE02" => Some(DbFormat::Locate02),
            _ => None,
        }
    }
}

fn match_entry(entry: &CStr, config: &Config, patterns: &Patterns) -> bool {
    let buf = Path::new(OsStr::from_bytes(entry.to_bytes()));
    let name = if config.basename {
        let Some(path) = buf.file_name() else {
            return false;
        };

        let c = CString::from_vec_with_nul(
            path.as_encoded_bytes()
                .iter()
                .copied()
                .chain([b'\0'])
                .collect(),
        )
        .unwrap();

        Cow::Owned(c)
    } else {
        Cow::Borrowed(entry)
    };
    let entry = name.to_string_lossy();

    (match config.all {
        false => {
            if entry.chars().any(|c| r"*?[]\".contains(c)) {
                // TODO: parse metacharacters
                false
            } else {
                patterns.any_match(entry.as_ref())
            }
        }
        true => {
            if entry.chars().any(|c| r"*?[]\".contains(c)) {
                // TODO: parse metacharacters
                false
            } else {
                patterns.all_match(entry.as_ref())
            }
        }
    }) && ((match config.existing {
        ExistenceMode::Any => true,
        ExistenceMode::Present => PathBuf::from(entry.to_string()).exists(),
        ExistenceMode::NotPresent => !PathBuf::from(entry.to_string()).exists(),
    }) || {
        if config.follow_symlinks {
            fs::symlink_metadata(PathBuf::from(entry.to_string())).is_ok()
        } else {
            false
        }
    })
}

fn do_locate(args: &[&str]) -> LocateResult<()> {
    let matches = uu_app().try_get_matches_from(args);
    match matches {
        Err(e) => {
            let mut app = uu_app();

            match e.kind() {
                clap::error::ErrorKind::DisplayHelp => {
                    app.print_help()?;
                }
                clap::error::ErrorKind::DisplayVersion => print!("{}", app.render_version()),
                _ => return Err(e.with_exit_code(1).into()),
            }
        }
        Ok(matches) => {
            let ParsedInfo { patterns, config } = ParsedInfo::from(matches);
            let mut stats = Statistics::new();

            // iterate over each given database
            let count = config
                .db
                .iter()
                .filter_map(|p| DbReader::new(p.as_path()).ok())
                .map(|mut dbreader| {
                    // if we can get the mtime of the file, check it against the current time
                    if let Ok(metadata) = fs::metadata(&dbreader.path) {
                        if let Ok(time) = metadata.modified() {
                            let modified: DateTime<Local> = time.into();
                            let now = Local::now();
                            let delta = now - modified;
                            if delta
                                > TimeDelta::days(config.max_age as i64)
                            {
                                eprintln!(
                                    "{}: warning: database ‘{}’ is more than {} days old (actual age is {:.1} days)",
                                    args[0],
                                    dbreader.path.to_string_lossy(),
                                    config.max_age,
                                    delta.num_seconds() as f64 / (60 * 60 * 24) as f64
                                );
                            }
                        }
                    }

                    // the first line of the statistics description is printed before matches
                    // (given --print)
                    if config.mode == Mode::Statistics {
                        stats.print_header(&dbreader);
                    }

                    // find matches
                    let count = dbreader
                        .by_ref()
                        .filter(|s| match_entry(s.as_c_str(), &config, &patterns))
                        .take(config.limit.unwrap_or(usize::MAX))
                        .inspect(|s| {
                            if config.mode == Mode::Normal || config.print {
                                if config.null_bytes {
                                    print!("{}\0", s.to_string_lossy());
                                } else {
                                println!("{}", s.to_string_lossy());}
                            }
                            if config.mode == Mode::Statistics {
                                stats.add_match(s);
                            }
                        })
                        .count();

                    // print the rest of the statistics description
                    if config.mode == Mode::Statistics {
                        stats.print(&dbreader);
                    }

                    count
                })
                .sum::<usize>();

            if config.mode == Mode::Count {
                println!("{count}");
            }

            // zero matches isn't an error if --statistics is passed
            if count == 0 && config.mode != Mode::Statistics {
                return Err(Error::NoMatches);
            }
        }
    }

    Ok(())
}

pub fn locate_main(args: &[&str]) -> i32 {
    match do_locate(args) {
        Ok(()) => 0,
        Err(e) => {
            match e {
                Error::NoMatches => {}
                _ => writeln!(&mut stderr(), "Error: {e}").unwrap(),
            }
            1
        }
    }
}
