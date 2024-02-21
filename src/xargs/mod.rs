// Copyright 2021 Collabora, Ltd.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::{
    collections::HashMap,
    error::Error,
    ffi::{OsStr, OsString},
    fmt::Display,
    fs,
    io::{self, BufRead, BufReader, Read},
    process::{Command, Stdio},
};

use clap::{crate_version, App, AppSettings, Arg};

mod options {
    pub const COMMAND: &str = "COMMAND";

    pub const ARG_FILE: &str = "arg-file";
    pub const DELIMITER: &str = "delimiter";
    pub const EXIT: &str = "exit";
    pub const MAX_ARGS: &str = "max-args";
    pub const MAX_CHARS: &str = "max-chars";
    pub const MAX_LINES: &str = "max-lines";
    pub const MAX_PROCS: &str = "max-procs";
    pub const NO_RUN_IF_EMPTY: &str = "no-run-if-empty";
    pub const NULL: &str = "null";
    pub const VERBOSE: &str = "verbose";
}

struct Options {
    arg_file: Option<String>,
    delimiter: Option<u8>,
    exit_if_pass_char_limit: bool,
    max_args: Option<usize>,
    max_chars: Option<usize>,
    max_lines: Option<usize>,
    no_run_if_empty: bool,
    null: bool,
    verbose: bool,
}

#[derive(Debug, PartialEq, Eq)]
enum ArgumentKind {
    /// An argument provided as part of the initial command line.
    Initial,
    /// An argument that was terminated by a newline or custom delimiter.
    HardTerminated,
    /// An argument that was terminated by non-newline whitespace.
    SoftTerminated,
}

#[derive(Debug, PartialEq, Eq)]
struct Argument {
    arg: OsString,
    kind: ArgumentKind,
}

struct ExhaustedCommandSpace {
    arg: Argument,
    out_of_chars: bool,
}

/// A "limiter" to constrain the size of a single command line. Given a cursor
/// pointing to the next limiter that should be tried.
trait CommandSizeLimiter {
    fn try_arg(
        &mut self,
        arg: Argument,
        cursor: LimiterCursor<'_>,
    ) -> Result<Argument, ExhaustedCommandSpace>;
    fn dyn_clone(&self) -> Box<dyn CommandSizeLimiter>;
}

/// A pointer to the next limiter. A limiter should *always* call the cursor's
/// try_next *before* updating its own state, to ensure that all other limiters
/// are okay with the argument first.
struct LimiterCursor<'collection> {
    limiters: &'collection mut [Box<dyn CommandSizeLimiter>],
}

impl LimiterCursor<'_> {
    fn try_next(self, arg: Argument) -> Result<Argument, ExhaustedCommandSpace> {
        if self.limiters.is_empty() {
            Ok(arg)
        } else {
            let (current, remaining) = self.limiters.split_at_mut(1);
            current[0].try_arg(
                arg,
                LimiterCursor {
                    limiters: remaining,
                },
            )
        }
    }
}

struct LimiterCollection {
    limiters: Vec<Box<dyn CommandSizeLimiter>>,
}

impl LimiterCollection {
    fn new() -> Self {
        Self { limiters: vec![] }
    }

    fn add(&mut self, limiter: impl CommandSizeLimiter + 'static) {
        self.limiters.push(Box::new(limiter));
    }

    fn try_arg(&mut self, arg: Argument) -> Result<Argument, ExhaustedCommandSpace> {
        let cursor = LimiterCursor {
            limiters: &mut self.limiters[..],
        };
        cursor.try_next(arg)
    }
}

impl Clone for LimiterCollection {
    fn clone(&self) -> Self {
        Self {
            limiters: self
                .limiters
                .iter()
                .map(|limiter| limiter.dyn_clone())
                .collect(),
        }
    }
}

#[cfg(windows)]
fn count_osstr_chars_for_exec(s: &OsStr) -> usize {
    use std::os::windows::ffi::OsStrExt;
    // Include +1 for either the null terminator or trailing space.
    s.encode_wide().count() + 1
}

#[cfg(unix)]
fn count_osstr_chars_for_exec(s: &OsStr) -> usize {
    use std::os::unix::ffi::OsStrExt;
    // Include +1 for the null terminator.
    s.as_bytes().len() + 1
}

#[derive(Clone)]
struct MaxCharsCommandSizeLimiter {
    current_size: usize,
    max_chars: usize,
}

impl MaxCharsCommandSizeLimiter {
    fn new(max_chars: usize) -> Self {
        Self {
            current_size: 0,
            max_chars,
        }
    }

    #[cfg(windows)]
    fn new_system(_env: &HashMap<OsString, OsString>) -> MaxCharsCommandSizeLimiter {
        // Taken from the CreateProcess docs.
        const MAX_CMDLINE: usize = 32767;
        MaxCharsCommandSizeLimiter::new(MAX_CMDLINE)
    }

    #[cfg(unix)]
    fn new_system(env: &HashMap<OsString, OsString>) -> Self {
        // POSIX requires that we leave 2048 bytes of space so that the child processes
        // can have room to set their own environment variables.
        const ARG_HEADROOM: usize = 2048;
        let arg_max = unsafe { uucore::libc::sysconf(uucore::libc::_SC_ARG_MAX) } as usize;

        let env_size: usize = env
            .iter()
            .map(|(var, value)| count_osstr_chars_for_exec(var) + count_osstr_chars_for_exec(value))
            .sum();

        Self::new(arg_max - ARG_HEADROOM - env_size)
    }
}

impl CommandSizeLimiter for MaxCharsCommandSizeLimiter {
    fn try_arg(
        &mut self,
        arg: Argument,
        cursor: LimiterCursor<'_>,
    ) -> Result<Argument, ExhaustedCommandSpace> {
        let chars = count_osstr_chars_for_exec(&arg.arg);
        if self.current_size + chars <= self.max_chars {
            let arg = cursor.try_next(arg)?;
            self.current_size += chars;
            Ok(arg)
        } else {
            Err(ExhaustedCommandSpace {
                arg,
                out_of_chars: true,
            })
        }
    }

    fn dyn_clone(&self) -> Box<dyn CommandSizeLimiter> {
        Box::new(self.clone())
    }
}

#[derive(Clone)]
struct MaxArgsCommandSizeLimiter {
    current_args: usize,
    max_args: usize,
}

impl MaxArgsCommandSizeLimiter {
    fn new(max_args: usize) -> Self {
        Self {
            current_args: 0,
            max_args,
        }
    }
}

impl CommandSizeLimiter for MaxArgsCommandSizeLimiter {
    fn try_arg(
        &mut self,
        arg: Argument,
        cursor: LimiterCursor<'_>,
    ) -> Result<Argument, ExhaustedCommandSpace> {
        if self.current_args < self.max_args {
            let arg = cursor.try_next(arg)?;
            if arg.kind != ArgumentKind::Initial {
                self.current_args += 1;
            }
            Ok(arg)
        } else {
            Err(ExhaustedCommandSpace {
                arg,
                out_of_chars: false,
            })
        }
    }

    fn dyn_clone(&self) -> Box<dyn CommandSizeLimiter> {
        Box::new(self.clone())
    }
}

#[derive(Clone)]
struct MaxLinesCommandSizeLimiter {
    current_line: usize,
    max_lines: usize,
}

impl MaxLinesCommandSizeLimiter {
    fn new(max_lines: usize) -> Self {
        Self {
            current_line: 1,
            max_lines,
        }
    }
}

impl CommandSizeLimiter for MaxLinesCommandSizeLimiter {
    fn try_arg(
        &mut self,
        arg: Argument,
        cursor: LimiterCursor<'_>,
    ) -> Result<Argument, ExhaustedCommandSpace> {
        if self.current_line <= self.max_lines {
            let arg = cursor.try_next(arg)?;
            // The name of this limiter is a bit of a lie: although this limits
            // by max "lines", if a custom delimiter is used, xargs uses that
            // instead. So, this actually limits based on the max amount of hard
            // terminations.
            if arg.kind == ArgumentKind::HardTerminated {
                self.current_line += 1;
            }
            Ok(arg)
        } else {
            Err(ExhaustedCommandSpace {
                arg,
                out_of_chars: false,
            })
        }
    }

    fn dyn_clone(&self) -> Box<dyn CommandSizeLimiter> {
        Box::new(self.clone())
    }
}

enum CommandResult {
    Success,
    Failure,
}

impl CommandResult {
    fn combine(&mut self, other: Self) {
        if matches!(*self, CommandResult::Success) {
            *self = other;
        }
    }
}

#[derive(Debug)]
enum CommandExecutionError {
    // exit code 255
    UrgentlyFailed,
    Killed { signal: i32 },
    CannotRun(io::Error),
    NotFound,
    Unknown,
}

impl Display for CommandExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandExecutionError::UrgentlyFailed => write!(f, "Command exited with code 255"),
            CommandExecutionError::Killed { signal } => {
                write!(f, "Command was killed with signal {signal}")
            }
            CommandExecutionError::CannotRun(err) => write!(f, "Command could not be run: {err}"),
            CommandExecutionError::NotFound => write!(f, "Command not found"),
            CommandExecutionError::Unknown => write!(f, "Unknown error running command"),
        }
    }
}

impl Error for CommandExecutionError {}

enum ExecAction {
    Command(Vec<OsString>),
    Echo,
}

struct CommandBuilderOptions {
    action: ExecAction,
    env: HashMap<OsString, OsString>,
    limiters: LimiterCollection,
    verbose: bool,
    close_stdin: bool,
}

impl CommandBuilderOptions {
    fn new(
        action: ExecAction,
        env: HashMap<OsString, OsString>,
        mut limiters: LimiterCollection,
    ) -> Result<Self, ExhaustedCommandSpace> {
        let initial_args = match &action {
            ExecAction::Command(args) => args.iter().map(|arg| arg.as_ref()).collect(),
            ExecAction::Echo => vec![OsStr::new("echo")],
        };

        for arg in initial_args {
            limiters.try_arg(Argument {
                arg: arg.to_owned(),
                kind: ArgumentKind::Initial,
            })?;
        }

        Ok(Self {
            action,
            env,
            limiters,
            verbose: false,
            close_stdin: false,
        })
    }
}

struct CommandBuilder<'options> {
    options: &'options CommandBuilderOptions,
    extra_args: Vec<OsString>,
    limiters: LimiterCollection,
}

impl CommandBuilder<'_> {
    fn new(options: &CommandBuilderOptions) -> CommandBuilder<'_> {
        CommandBuilder {
            options,
            extra_args: vec![],
            limiters: options.limiters.clone(),
        }
    }

    fn add_arg(&mut self, arg: Argument) -> Result<(), ExhaustedCommandSpace> {
        let arg = self.limiters.try_arg(arg)?;
        self.extra_args.push(arg.arg);
        Ok(())
    }

    fn execute(self) -> Result<CommandResult, CommandExecutionError> {
        let (entry_point, initial_args): (&OsStr, &[OsString]) = match &self.options.action {
            ExecAction::Command(args) => (&args[0], &args[1..]),
            ExecAction::Echo => (OsStr::new("echo"), &[]),
        };

        let mut command = Command::new(entry_point);
        command
            .args(initial_args)
            .args(&self.extra_args)
            .env_clear()
            .envs(&self.options.env);
        if self.options.close_stdin {
            command.stdin(Stdio::null());
        }
        if self.options.verbose {
            eprintln!("{command:?}");
        }

        match &self.options.action {
            ExecAction::Command(_) => match command.status() {
                Ok(status) => {
                    if status.success() {
                        Ok(CommandResult::Success)
                    } else if let Some(err) = status.code() {
                        if err == 255 {
                            Err(CommandExecutionError::UrgentlyFailed)
                        } else {
                            Ok(CommandResult::Failure)
                        }
                    } else {
                        #[cfg(unix)]
                        {
                            use std::os::unix::process::ExitStatusExt;
                            if let Some(signal) = status.signal() {
                                Err(CommandExecutionError::Killed { signal })
                            } else {
                                Err(CommandExecutionError::Unknown)
                            }
                        }

                        #[cfg(not(unix))]
                        Err(CommandExecutionError::Unknown)
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::NotFound => {
                    Err(CommandExecutionError::NotFound)
                }
                Err(e) => Err(CommandExecutionError::CannotRun(e)),
            },
            ExecAction::Echo => {
                println!(
                    "{}",
                    self.extra_args
                        .iter()
                        .map(|arg| arg.to_string_lossy())
                        .collect::<Vec<_>>()
                        .join(" ")
                );
                Ok(CommandResult::Success)
            }
        }
    }
}

trait ArgumentReader {
    fn next(&mut self) -> io::Result<Option<Argument>>;
}

struct WhitespaceDelimitedArgumentReader<R: Read> {
    rd: R,
    pending: Vec<u8>,
}

impl<R> WhitespaceDelimitedArgumentReader<R>
where
    R: Read,
{
    fn new(rd: R) -> Self {
        Self {
            rd,
            pending: vec![],
        }
    }
}

impl<R> ArgumentReader for WhitespaceDelimitedArgumentReader<R>
where
    R: Read,
{
    fn next(&mut self) -> io::Result<Option<Argument>> {
        let mut result = vec![];
        let mut terminated_by_newline = false;

        let mut pending = vec![];
        std::mem::swap(&mut pending, &mut self.pending);

        enum Escape {
            Slash,
            Quote(u8),
        }

        let mut escape: Option<Escape> = None;
        let mut i = 0;
        loop {
            if i == pending.len() {
                pending.resize(4096, 0);
                // Already hit the end of our buffer, so read in some more data.
                let bytes_read = loop {
                    match self.rd.read(&mut pending[..]) {
                        Ok(bytes_read) => break bytes_read,
                        Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                        Err(e) => return Err(e),
                    }
                };

                if bytes_read == 0 {
                    if let Some(Escape::Quote(q)) = &escape {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            format!("Unterminated quote: {q}"),
                        ));
                    }
                    if i == 0 {
                        return Ok(None);
                    }
                    pending.clear();
                    break;
                }

                pending.resize(bytes_read, 0);
                i = 0;
            }

            match (&escape, pending[i]) {
                (Some(Escape::Quote(quote)), c) if c == *quote => escape = None,
                (Some(Escape::Quote(_)), c) => result.push(c),
                (Some(Escape::Slash), c) => {
                    result.push(c);
                    escape = None;
                }
                (None, c @ (b'"' | b'\'')) => escape = Some(Escape::Quote(c)),
                (None, b'\\') => escape = Some(Escape::Slash),
                (None, c) if c.is_ascii_whitespace() => {
                    if !result.is_empty() {
                        terminated_by_newline = c == b'\n';
                        break;
                    }
                }
                (None, c) => result.push(c),
            }

            i += 1;
        }

        if i < pending.len() {
            self.pending = pending.split_off(i + 1);
        }

        Ok(Some(Argument {
            arg: String::from_utf8_lossy(&result[..]).into_owned().into(),
            kind: if terminated_by_newline {
                ArgumentKind::HardTerminated
            } else {
                ArgumentKind::SoftTerminated
            },
        }))
    }
}

struct ByteDelimitedArgumentReader<R: Read> {
    rd: BufReader<R>,
    delimiter: u8,
}

impl<R> ByteDelimitedArgumentReader<R>
where
    R: Read,
{
    fn new(rd: R, delimiter: u8) -> Self {
        Self {
            rd: BufReader::new(rd),
            delimiter,
        }
    }
}

impl<R> ArgumentReader for ByteDelimitedArgumentReader<R>
where
    R: Read,
{
    fn next(&mut self) -> io::Result<Option<Argument>> {
        Ok(loop {
            let mut buf = vec![];
            let bytes_read = self.rd.read_until(self.delimiter, &mut buf)?;
            if bytes_read > 0 {
                let need_to_trim_delimiter = buf[buf.len() - 1] == self.delimiter;
                let bytes = if need_to_trim_delimiter {
                    if buf.len() == 1 {
                        // This was *only* a delimiter, so we didn't actually
                        // read anything interesting. Try again.
                        continue;
                    }

                    &buf[..buf.len() - 1]
                } else {
                    &buf[..]
                };
                break Some(Argument {
                    arg: String::from_utf8_lossy(bytes).into_owned().into(),
                    kind: ArgumentKind::HardTerminated,
                });
            }
            break None;
        })
    }
}

#[derive(Debug)]
enum XargsError {
    ArgumentTooLarge,
    CommandExecution(CommandExecutionError),
    Io(io::Error),
    Untyped(String),
}

impl Display for XargsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            XargsError::ArgumentTooLarge => write!(f, "Argument too large"),
            XargsError::CommandExecution(e) => write!(f, "{e}"),
            XargsError::Io(e) => write!(f, "{e}"),
            XargsError::Untyped(s) => write!(f, "{s}"),
        }
    }
}

impl Error for XargsError {}

impl From<String> for XargsError {
    fn from(s: String) -> Self {
        Self::Untyped(s)
    }
}

impl From<&'_ str> for XargsError {
    fn from(s: &'_ str) -> Self {
        s.to_owned().into()
    }
}

impl From<CommandExecutionError> for XargsError {
    fn from(e: CommandExecutionError) -> Self {
        Self::CommandExecution(e)
    }
}

impl From<io::Error> for XargsError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

fn process_input(
    builder_options: CommandBuilderOptions,
    mut args: Box<dyn ArgumentReader>,
    options: &Options,
) -> Result<CommandResult, XargsError> {
    let mut current_builder = CommandBuilder::new(&builder_options);
    let mut have_pending_command = false;
    let mut result = CommandResult::Success;

    while let Some(arg) = args.next()? {
        if let Err(ExhaustedCommandSpace { arg, out_of_chars }) = current_builder.add_arg(arg) {
            if out_of_chars
                && options.exit_if_pass_char_limit
                && (options.max_args.is_some() || options.max_lines.is_some())
            {
                return Err(XargsError::ArgumentTooLarge);
            }
            if have_pending_command {
                result.combine(current_builder.execute()?);
            }

            current_builder = CommandBuilder::new(&builder_options);
            if let Err(ExhaustedCommandSpace { .. }) = current_builder.add_arg(arg) {
                return Err(XargsError::ArgumentTooLarge);
            }
        }

        have_pending_command = true;
    }

    if !options.no_run_if_empty || have_pending_command {
        result.combine(current_builder.execute()?);
    }

    Ok(result)
}

fn parse_delimiter(s: &str) -> Result<u8, String> {
    if let Some(hex) = s.strip_prefix("\\x") {
        u8::from_str_radix(hex, 16).map_err(|e| format!("Invalid hex sequence: {}", e))
    } else if let Some(oct) = s.strip_prefix("\\0") {
        u8::from_str_radix(oct, 8).map_err(|e| format!("Invalid octal sequence: {}", e))
    } else if let Some(special) = s.strip_prefix('\\') {
        match special {
            "a" => Ok(b'\x07'),
            "b" => Ok(b'\x08'),
            "f" => Ok(b'\x0C'),
            "n" => Ok(b'\n'),
            "r" => Ok(b'\r'),
            "t" => Ok(b'\t'),
            "v" => Ok(b'\x0B'),
            "0" => Ok(b'\0'),
            "\\" => Ok(b'\\'),
            _ => Err(format!("Invalid escape sequence: {s}")),
        }
    } else {
        let bytes = s.as_bytes();
        if bytes.len() == 1 {
            Ok(bytes[0])
        } else {
            Err("Delimiter must be one byte".to_owned())
        }
    }
}

fn validate_positive_usize(s: String) -> Result<(), String> {
    match s.parse::<usize>() {
        Ok(v) if v > 0 => Ok(()),
        Ok(v) => Err(format!("Value must be > 0, not: {v}")),
        Err(e) => Err(e.to_string()),
    }
}

fn do_xargs(args: &[&str]) -> Result<CommandResult, XargsError> {
    let matches = App::new("xargs")
        .version(crate_version!())
        .about("Run commands using arguments derived from standard input")
        .settings(&[AppSettings::TrailingVarArg])
        .arg(
            Arg::with_name(options::COMMAND)
                .takes_value(true)
                .multiple(true)
                .help("The command to run"),
        )
        .arg(
            Arg::with_name(options::ARG_FILE)
                .short("a")
                .long(options::ARG_FILE)
                .takes_value(true)
                .help("Read arguments from the given file instead of stdin"),
        )
        .arg(
            Arg::with_name(options::DELIMITER)
                .short("d")
                .long(options::DELIMITER)
                .takes_value(true)
                .validator(|s| parse_delimiter(&s).map(|_| ()))
                .help("Use the given delimiter to split the input"),
        )
        .arg(
            Arg::with_name(options::EXIT)
                .short("x")
                .long(options::EXIT)
                .help(
                    "Exit if the number of arguments allowed by -L or -n do not \
                    fit into the number of allowed characters",
                ),
        )
        .arg(
            Arg::with_name(options::MAX_ARGS)
                .short("n")
                .takes_value(true)
                .long(options::MAX_ARGS)
                .validator(validate_positive_usize)
                .help(
                    "Set the max number of arguments read from stdin to be passed \
                    to each command invocation (mutually exclusive with -L)",
                ),
        )
        .arg(
            Arg::with_name(options::MAX_LINES)
                .short("L")
                .takes_value(true)
                .validator(validate_positive_usize)
                .help(
                    "Set the max number of lines from stdin to be passed to each \
                    command invocation (mutually exclusive with -n)",
                ),
        )
        .arg(
            Arg::with_name(options::MAX_PROCS)
                .short("P")
                .takes_value(true)
                .long(options::MAX_PROCS)
                .help("Run up to this many commands in parallel [NOT IMPLEMENTED]"),
        )
        .arg(
            Arg::with_name(options::NO_RUN_IF_EMPTY)
                .short("r")
                .long(options::NO_RUN_IF_EMPTY)
                .help("If there are no input arguments, do not run the command at all"),
        )
        .arg(
            Arg::with_name(options::NULL)
                .short("0")
                .long(options::NULL)
                .help("Split the input by null terminators rather than whitespace"),
        )
        .arg(
            Arg::with_name(options::MAX_CHARS)
                .short("s")
                .long(options::MAX_CHARS)
                .takes_value(true)
                .validator(validate_positive_usize)
                .help(
                    "Set the max number of characters to be passed to each \
                    invocation",
                ),
        )
        .arg(
            Arg::with_name(options::VERBOSE)
                .short("t")
                .long(options::VERBOSE)
                .help("Be verbose"),
        )
        .get_matches_from(args);

    let options = Options {
        arg_file: matches
            .value_of(options::ARG_FILE)
            .map(|value| value.to_owned()),
        delimiter: matches
            .value_of(options::DELIMITER)
            .map(|value| parse_delimiter(value).unwrap()),
        exit_if_pass_char_limit: matches.is_present(options::EXIT),
        max_args: matches
            .value_of(options::MAX_ARGS)
            .map(|value| value.parse().unwrap()),
        max_chars: matches
            .value_of(options::MAX_CHARS)
            .map(|value| value.parse().unwrap()),
        max_lines: matches
            .value_of(options::MAX_LINES)
            .map(|value| value.parse().unwrap()),
        no_run_if_empty: matches.is_present(options::NO_RUN_IF_EMPTY),
        null: matches.is_present(options::NULL),
        verbose: matches.is_present(options::VERBOSE),
    };

    let delimiter = match (options.delimiter, options.null) {
        (Some(delimiter), true) => {
            if matches.indices_of(options::NULL).unwrap().last()
                > matches.indices_of(options::DELIMITER).unwrap().last()
            {
                Some(b'\0')
            } else {
                Some(delimiter)
            }
        }
        (Some(delimiter), false) => Some(delimiter),
        (None, true) => Some(b'\0'),
        (None, false) => None,
    };

    let action = match matches.values_of_os(options::COMMAND) {
        Some(args) if args.len() > 0 => {
            ExecAction::Command(args.map(|arg| arg.to_owned()).collect())
        }
        _ => ExecAction::Echo,
    };

    let env = std::env::vars_os().collect();

    let mut limiters = LimiterCollection::new();

    match (options.max_args, options.max_lines) {
        (Some(max_args), Some(max_lines)) => {
            eprintln!(
                "WARNING: Both --{} and -L were given; last option will be used",
                options::MAX_ARGS,
            );
            if matches.indices_of(options::MAX_LINES).unwrap().last()
                > matches.indices_of(options::MAX_ARGS).unwrap().last()
            {
                limiters.add(MaxLinesCommandSizeLimiter::new(max_lines));
            } else {
                limiters.add(MaxArgsCommandSizeLimiter::new(max_args));
            }
        }
        (Some(max_args), None) => limiters.add(MaxArgsCommandSizeLimiter::new(max_args)),
        (None, Some(max_lines)) => limiters.add(MaxLinesCommandSizeLimiter::new(max_lines)),
        (None, None) => (),
    }

    if let Some(max_chars) = options.max_chars {
        limiters.add(MaxCharsCommandSizeLimiter::new(max_chars));
    }

    limiters.add(MaxCharsCommandSizeLimiter::new_system(&env));

    let mut builder_options = CommandBuilderOptions::new(action, env, limiters).map_err(|_| {
        "Base command and environment are too large to fit into one command execution"
    })?;

    builder_options.verbose = options.verbose;
    builder_options.close_stdin = options.arg_file.is_none();

    let args_file: Box<dyn Read> = if let Some(path) = &options.arg_file {
        Box::new(fs::File::open(path).map_err(|e| format!("Failed to open {}: {}", path, e))?)
    } else {
        Box::new(io::stdin())
    };

    let args: Box<dyn ArgumentReader> = if let Some(delimiter) = delimiter {
        Box::new(ByteDelimitedArgumentReader::new(args_file, delimiter))
    } else {
        Box::new(WhitespaceDelimitedArgumentReader::new(args_file))
    };

    let result = process_input(builder_options, args, &options)?;
    Ok(result)
}

pub fn xargs_main(args: &[&str]) -> i32 {
    match do_xargs(args) {
        Ok(CommandResult::Success) => 0,
        Ok(CommandResult::Failure) => 123,
        Err(e) => {
            eprintln!("Error: {e}");
            if let XargsError::CommandExecution(cx) = e {
                match cx {
                    CommandExecutionError::UrgentlyFailed => 124,
                    CommandExecutionError::Killed { .. } => 125,
                    CommandExecutionError::CannotRun(_) => 126,
                    CommandExecutionError::NotFound => 127,
                    CommandExecutionError::Unknown => 1,
                }
            } else {
                1
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_arg_init(s: &str) -> Argument {
        Argument {
            arg: s.to_owned().into(),
            kind: ArgumentKind::Initial,
        }
    }

    fn make_arg_hard(s: &str) -> Argument {
        Argument {
            arg: s.to_owned().into(),
            kind: ArgumentKind::HardTerminated,
        }
    }

    fn make_arg_soft(s: &str) -> Argument {
        Argument {
            arg: s.to_owned().into(),
            kind: ArgumentKind::SoftTerminated,
        }
    }

    #[derive(Clone)]
    struct AlwaysRejectLimiter;

    impl CommandSizeLimiter for AlwaysRejectLimiter {
        fn try_arg(
            &mut self,
            arg: Argument,
            _cursor: LimiterCursor<'_>,
        ) -> Result<Argument, ExhaustedCommandSpace> {
            Err(ExhaustedCommandSpace {
                arg,
                out_of_chars: false,
            })
        }

        fn dyn_clone(&self) -> Box<dyn CommandSizeLimiter> {
            Box::new(self.clone())
        }
    }

    fn empty_cursor() -> LimiterCursor<'static> {
        LimiterCursor { limiters: &mut [] }
    }

    enum Chunk {
        Data(&'static [u8]),
        Error(io::ErrorKind),
    }

    struct ChunkReader {
        chunks: Vec<Chunk>,
        current: usize,
    }

    impl ChunkReader {
        fn new(chunks: Vec<Chunk>) -> Self {
            Self { chunks, current: 0 }
        }
    }

    impl Read for ChunkReader {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.current >= self.chunks.len() {
                return Ok(0);
            }

            match &mut self.chunks[self.current] {
                Chunk::Data(data) => {
                    let byte_count = std::cmp::min(data.len(), buf.len());
                    buf[..byte_count].copy_from_slice(&(*data)[..byte_count]);
                    if byte_count == data.len() {
                        self.current += 1;
                    } else {
                        *data = &(*data)[byte_count..];
                    }

                    Ok(byte_count)
                }
                Chunk::Error(kind) => {
                    self.current += 1;
                    Err(io::Error::new(*kind, "Synthesized error"))
                }
            }
        }
    }

    #[test]
    fn test_chars_limiter() {
        let mut limiter = MaxCharsCommandSizeLimiter::new(6);
        assert!(limiter
            .try_arg(make_arg_hard("abc"), empty_cursor())
            .is_ok());
        assert!(limiter
            .try_arg(make_arg_hard("abcd"), empty_cursor())
            .is_err());
        assert!(limiter.try_arg(make_arg_hard("a"), empty_cursor()).is_ok());
    }

    #[test]
    fn test_chars_limiter_asks_cursor() {
        let mut rejects: [Box<dyn CommandSizeLimiter>; 1] = [Box::new(AlwaysRejectLimiter)];
        let reject_cursor = LimiterCursor {
            limiters: &mut rejects,
        };

        let mut limiter = MaxCharsCommandSizeLimiter::new(5);
        assert!(limiter
            .try_arg(make_arg_hard("abc"), reject_cursor)
            .is_err());
        // Ensure the limiter didn't update before trying the cursor.
        assert!(limiter
            .try_arg(make_arg_hard("abc"), empty_cursor())
            .is_ok());
    }

    #[test]
    fn test_args_limiter() {
        let mut limiter = MaxArgsCommandSizeLimiter::new(2);
        // Should not count initial arguments.
        for _ in 1..3 {
            assert!(limiter
                .try_arg(make_arg_init("abc"), empty_cursor())
                .is_ok());
        }
        assert!(limiter
            .try_arg(make_arg_hard("abc"), empty_cursor())
            .is_ok());
        assert!(limiter
            .try_arg(make_arg_hard("abc"), empty_cursor())
            .is_ok());
        assert!(limiter
            .try_arg(make_arg_hard("abc"), empty_cursor())
            .is_err());
    }

    #[test]
    fn test_args_limiter_asks_cursor() {
        let mut rejects: [Box<dyn CommandSizeLimiter>; 1] = [Box::new(AlwaysRejectLimiter)];
        let reject_cursor = LimiterCursor {
            limiters: &mut rejects,
        };

        let mut limiter = MaxArgsCommandSizeLimiter::new(1);
        assert!(limiter
            .try_arg(make_arg_hard("abc"), reject_cursor)
            .is_err());
        // Ensure the limiter didn't update before trying the cursor.
        assert!(limiter
            .try_arg(make_arg_hard("abc"), empty_cursor())
            .is_ok());
    }

    #[test]
    fn test_lines_limiter() {
        let mut limiter = MaxLinesCommandSizeLimiter::new(2);
        assert!(limiter
            .try_arg(make_arg_soft("abc"), empty_cursor())
            .is_ok());
        assert!(limiter
            .try_arg(make_arg_soft("abc"), empty_cursor())
            .is_ok());
        assert!(limiter
            .try_arg(make_arg_soft("abc"), empty_cursor())
            .is_ok());
        assert!(limiter
            .try_arg(make_arg_hard("abc"), empty_cursor())
            .is_ok());
        assert!(limiter
            .try_arg(make_arg_soft("abc"), empty_cursor())
            .is_ok());
        assert!(limiter
            .try_arg(make_arg_hard("abc"), empty_cursor())
            .is_ok());
        assert!(limiter
            .try_arg(make_arg_soft("abc"), empty_cursor())
            .is_err());
        assert!(limiter
            .try_arg(make_arg_hard("abc"), empty_cursor())
            .is_err());
    }

    #[test]
    fn test_lines_limiter_asks_cursor() {
        let mut rejects: [Box<dyn CommandSizeLimiter>; 1] = [Box::new(AlwaysRejectLimiter)];
        let reject_cursor = LimiterCursor {
            limiters: &mut rejects,
        };

        let mut limiter = MaxLinesCommandSizeLimiter::new(1);
        assert!(limiter
            .try_arg(make_arg_hard("abc"), reject_cursor)
            .is_err());
        // Ensure the limiter didn't update before trying the cursor.
        assert!(limiter
            .try_arg(make_arg_hard("abc"), empty_cursor())
            .is_ok());
    }

    #[test]
    fn test_whitespace_delimited_reader() {
        let mut reader = WhitespaceDelimitedArgumentReader::new(ChunkReader::new(vec![
            Chunk::Data(b"abc "),
            Chunk::Data(b" def"),
            Chunk::Data(b"\nghi\t\tj"),
            Chunk::Data(b"kl\n"),
            Chunk::Data(b"mn"),
            Chunk::Error(io::ErrorKind::Interrupted),
            Chunk::Data(b"\\\t\\ o 'ab"),
            Chunk::Data(b" \"' \"xy' z\""),
        ]));

        assert_eq!(reader.next().unwrap().unwrap(), make_arg_soft("abc"));
        assert_eq!(reader.next().unwrap().unwrap(), make_arg_hard("def"));
        assert_eq!(reader.next().unwrap().unwrap(), make_arg_soft("ghi"));
        assert_eq!(reader.next().unwrap().unwrap(), make_arg_hard("jkl"));
        assert_eq!(reader.next().unwrap().unwrap(), make_arg_soft("mn\t o"));
        assert_eq!(reader.next().unwrap().unwrap(), make_arg_soft("ab \""));
        assert_eq!(reader.next().unwrap().unwrap(), make_arg_soft("xy' z"));
        assert_eq!(reader.next().unwrap(), None);
    }

    #[test]
    fn test_byte_delimited_reader() {
        let mut reader = ByteDelimitedArgumentReader::new(
            ChunkReader::new(vec![
                Chunk::Data(b"ab"),
                Chunk::Error(io::ErrorKind::Interrupted),
                Chunk::Data(b"c!de!"),
                Chunk::Data(b"!ef!!gh"),
                Chunk::Data(b"!ij"),
            ]),
            b'!',
        );

        assert_eq!(reader.next().unwrap().unwrap(), make_arg_hard("abc"));
        assert_eq!(reader.next().unwrap().unwrap(), make_arg_hard("de"));
        assert_eq!(reader.next().unwrap().unwrap(), make_arg_hard("ef"));
        assert_eq!(reader.next().unwrap().unwrap(), make_arg_hard("gh"));
        assert_eq!(reader.next().unwrap().unwrap(), make_arg_hard("ij"));
        assert_eq!(reader.next().unwrap(), None);
    }

    #[test]
    fn test_delimiter_parsing() {
        assert_eq!(parse_delimiter("a").unwrap(), b'a');
        assert_eq!(parse_delimiter("\\x61").unwrap(), b'a');
        assert_eq!(parse_delimiter("\\x00061").unwrap(), b'a');
        assert_eq!(parse_delimiter("\\0141").unwrap(), b'a');
        assert_eq!(parse_delimiter("\\0000141").unwrap(), b'a');
        assert_eq!(parse_delimiter("\\n").unwrap(), b'\n');

        assert!(parse_delimiter("\\0").is_err());
        assert!(parse_delimiter("\\x").is_err());
        assert!(parse_delimiter("\\").is_err());
        assert!(parse_delimiter("abc").is_err());
    }
}
