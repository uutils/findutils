// Copyright 2021 Collabora, Ltd.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::borrow::Cow;
use std::error::Error;
use std::fs::{self, File};
use std::io::{stderr, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{format::StrftimeItems, DateTime, Local};

use super::{FileType, Matcher, MatcherIO, StdoutErrors, WalkEntry, WalkError};

#[cfg(unix)]
use std::os::unix::prelude::MetadataExt;

#[cfg(unix)]
const STANDARD_BLOCK_SIZE: u64 = 512;

#[derive(Debug, PartialEq, Eq)]
enum Justify {
    Left,
    Right,
}

/// The `+`, space, `#` and `0` flags of a directive. Like GNU find, they only
/// change the numeric directives `%d` and `%m`.
#[derive(Debug, Default, PartialEq, Eq)]
struct Flags {
    plus: bool,
    space: bool,
    alternate: bool,
    zero: bool,
}

#[derive(Debug, PartialEq, Eq)]
enum TimeFormat {
    /// Follow ctime(3).
    Ctime,
    /// Seconds since the epoch, as a float w/ nanosecond part.
    SinceEpoch,
    /// Follow strftime-compatible syntax
    Strftime(String),
}

impl TimeFormat {
    fn apply(&self, time: SystemTime) -> Result<Cow<'static, str>, Box<dyn Error>> {
        let formatted = match self {
            Self::SinceEpoch => {
                let duration = time.duration_since(SystemTime::UNIX_EPOCH)?;
                format!("{}.{:09}0", duration.as_secs(), duration.subsec_nanos())
            }
            Self::Ctime => {
                const CTIME_FORMAT: &str = "%a %b %e %H:%M:%S.%f0 %Y";

                DateTime::<Local>::from(time)
                    .format(CTIME_FORMAT)
                    .to_string()
            }
            Self::Strftime(format) => {
                // Handle a special case
                // GNU prints a fixed-width fraction: dot, nine nanosecond
                // digits, plus a trailing literal zero. chrono's `%.f` would
                // drop the fraction (and the dot) entirely when it is zero.
                let custom_format = format.replace("%+", "%Y-%m-%d+%H:%M:%S.%f0");
                DateTime::<Local>::from(time)
                    .format(&custom_format)
                    .to_string()
            }
        };

        Ok(formatted.into())
    }
}

#[derive(Debug, PartialEq, Eq)]
enum PermissionsFormat {
    Octal,
    // trwxrwxrwx
    Symbolic,
}

/// A single % directive in a format string.
#[derive(Debug, PartialEq, Eq)]
enum FormatDirective {
    // %a, %Ak
    AccessTime(TimeFormat),
    // %b, %k
    Blocks { large_blocks: bool },
    // %c, %Ck
    ChangeTime(TimeFormat),
    // %d
    Depth,
    // %D
    Device,
    // %f
    Basename,
    // %F
    Filesystem,
    // %g, %G
    Group { as_name: bool },
    // %h
    Dirname,
    // %H
    StartingPoint,
    // %i
    Inode,
    // %l
    SymlinkTarget,
    // %m
    Permissions(PermissionsFormat),
    // %n
    HardlinkCount,
    // %p, %P
    Path { strip_starting_point: bool },
    // %s
    Size,
    // %S
    Sparseness,
    // %t, %Tk
    ModificationTime(TimeFormat),
    // %u, %U
    User { as_name: bool },
    // %y, %Y
    Type { follow_links: bool },
}

/// A component in a full format string.
#[derive(Debug, PartialEq, Eq)]
enum FormatComponent {
    Literal(String),
    Byte(u8),
    Flush,
    Directive {
        directive: FormatDirective,
        width: Option<usize>,
        precision: Option<usize>,
        justify: Justify,
        flags: Flags,
    },
}

struct FormatStringParser<'a> {
    string: &'a str,
}

impl FormatStringParser<'_> {
    fn front(&self) -> Result<char, Box<dyn Error>> {
        self.string
            .chars()
            .next()
            .ok_or_else(|| "Unexpected EOF".into())
    }

    fn peek(&self, count: usize) -> Result<&str, Box<dyn Error>> {
        if self.string.len() < count {
            return Err("Unexpected EOF".into());
        }

        Ok(&self.string[0..count])
    }

    fn advance_one(&mut self) -> Result<char, Box<dyn Error>> {
        let c = self.front()?;
        self.string = &self.string[c.len_utf8()..];
        Ok(c)
    }

    fn advance_by(&mut self, count: usize) -> Result<&str, Box<dyn Error>> {
        self.peek(count)?;

        let skipped = &self.string[0..count];
        self.string = &self.string[count..];
        Ok(skipped)
    }

    fn parse_escape_sequence(&mut self) -> Result<FormatComponent, Box<dyn Error>> {
        const OCTAL_LEN: usize = 3;
        const OCTAL_RADIX: u32 = 8;

        if self.string.is_empty() {
            eprintln!(
                "{}: warning: escape `\\' followed by nothing at all",
                crate::find::program_name()
            );
            return Ok(FormatComponent::Literal("\\".to_owned()));
        }

        // Try parsing an octal sequence first.
        let first = self.front()?;
        if first.is_digit(OCTAL_RADIX) {
            // A GNU octal escape is 1 to 3 octal digits. Consume only the leading
            // octal digits (which are ASCII), rather than slicing a fixed 3 bytes
            // that can land inside a following multibyte char.
            let octal: String = self
                .string
                .chars()
                .take(OCTAL_LEN)
                .take_while(|c| c.is_digit(OCTAL_RADIX))
                .collect();
            let code = u32::from_str_radix(&octal, OCTAL_RADIX)?;
            self.advance_by(octal.len())?;
            // GNU octal escapes emit one byte, truncating values above 0xff.
            return Ok(FormatComponent::Byte(code as u8));
        }

        self.advance_one()?;

        if first == 'c' {
            Ok(FormatComponent::Flush)
        } else {
            let c = match first {
                'a' => "\x07",
                'b' => "\x08",
                'f' => "\x0C",
                'n' => "\n",
                'r' => "\r",
                't' => "\t",
                'v' => "\x0B",
                '0' => "\0",
                '\\' => "\\",
                c => {
                    eprintln!(
                        "{}: warning: unrecognized escape '\\{c}'",
                        crate::find::program_name()
                    );
                    return Ok(FormatComponent::Literal(format!("\\{c}")));
                }
            };

            Ok(FormatComponent::Literal(c.to_string()))
        }
    }

    fn parse_format_width(&mut self) -> Result<Option<usize>, Box<dyn Error>> {
        let width = self.parse_format_number()?;
        match width {
            Some(width) if width > u16::MAX as usize => {
                Err(format!("Format width too large: {width}").into())
            }
            _ => Ok(width),
        }
    }

    fn parse_format_number(&mut self) -> Result<Option<usize>, Box<dyn Error>> {
        let start = self.string;
        let mut digits = 0;

        while self.front().is_ok_and(|c| c.is_ascii_digit()) {
            digits += 1;
            // safe to unwrap: the front() check already succeeded above.
            self.advance_one().unwrap();
        }

        if digits > 0 {
            let digits = &start[0..digits];
            let width: usize = digits
                .parse()
                .map_err(|_| format!("Invalid format width: {digits}"))?;
            Ok(Some(width))
        } else {
            Ok(None)
        }
    }

    fn parse_time_specifier(&mut self, first: char) -> Result<TimeFormat, Box<dyn Error>> {
        match self.advance_one()? {
            '@' => Ok(TimeFormat::SinceEpoch),
            'S' => Ok(TimeFormat::Strftime("%S.%f0".to_string())),
            c => {
                // We can't store the parsed items inside TimeFormat, because the items
                // take a reference to the full format string, but we still try to parse
                // it here so that errors get caught early.
                let format = format!("%{c}");
                match StrftimeItems::new(&format).next() {
                    None | Some(chrono::format::Item::Error) => {
                        Err(format!("Invalid time specifier: %{first}{c}").into())
                    }
                    Some(_item) => Ok(TimeFormat::Strftime(format)),
                }
            }
        }
    }

    fn parse_format_specifier(&mut self) -> Result<FormatComponent, Box<dyn Error>> {
        let start = self.string;
        let mut justify = Justify::Right;
        let mut flags = Flags::default();
        loop {
            match self.front()? {
                ' ' => flags.space = true,
                '+' => flags.plus = true,
                '#' => flags.alternate = true,
                '-' => justify = Justify::Left,
                _ => break,
            }

            // safe to unwrap: .front() already succeeded above.
            self.advance_one().unwrap();
        }
        // GNU find only takes `0` after the other flags.
        if self.front()? == '0' {
            flags.zero = true;
            self.advance_one()?;
        }

        let width = self.parse_format_width()?;
        let precision = if self.front()? == '.' {
            self.advance_one()?;
            Some(self.parse_format_number()?.unwrap_or(0))
        } else {
            None
        };

        let first = self.advance_one()?;
        if first == '%' {
            return Ok(FormatComponent::Literal("%".to_owned()));
        }

        let directive = match first {
            'a' => FormatDirective::AccessTime(TimeFormat::Ctime),
            'A' => FormatDirective::AccessTime(self.parse_time_specifier(first)?),
            'b' => FormatDirective::Blocks {
                large_blocks: false,
            },
            'c' => FormatDirective::ChangeTime(TimeFormat::Ctime),
            'C' => FormatDirective::ChangeTime(self.parse_time_specifier(first)?),
            'd' => FormatDirective::Depth,
            'D' => FormatDirective::Device,
            'f' => FormatDirective::Basename,
            'F' => FormatDirective::Filesystem,
            'g' => FormatDirective::Group { as_name: true },
            'G' => FormatDirective::Group { as_name: false },
            'h' => FormatDirective::Dirname,
            'H' => FormatDirective::StartingPoint,
            'k' => FormatDirective::Blocks { large_blocks: true },
            'i' => FormatDirective::Inode,
            'l' => FormatDirective::SymlinkTarget,
            'm' => FormatDirective::Permissions(PermissionsFormat::Octal),
            'M' => FormatDirective::Permissions(PermissionsFormat::Symbolic),
            'n' => FormatDirective::HardlinkCount,
            'p' => FormatDirective::Path {
                strip_starting_point: false,
            },
            'P' => FormatDirective::Path {
                strip_starting_point: true,
            },
            's' => FormatDirective::Size,
            'S' => FormatDirective::Sparseness,
            't' => FormatDirective::ModificationTime(TimeFormat::Ctime),
            'T' => FormatDirective::ModificationTime(self.parse_time_specifier(first)?),
            'u' => FormatDirective::User { as_name: true },
            'U' => FormatDirective::User { as_name: false },
            'y' => FormatDirective::Type {
                follow_links: false,
            },
            'Y' => FormatDirective::Type { follow_links: true },
            // TODO: %Z
            _ => {
                eprintln!(
                    "{}: warning: unrecognized format directive '%{first}'",
                    crate::find::program_name()
                );
                let text = &start[..start.len() - self.string.len()];
                return Ok(FormatComponent::Literal(format!("%{text}")));
            }
        };

        Ok(FormatComponent::Directive {
            directive,
            width,
            precision,
            justify,
            flags,
        })
    }

    pub fn parse(&mut self) -> Result<FormatString, Box<dyn Error>> {
        let mut components = vec![];

        while let Some(i) = self.string.find(['%', '\\']) {
            if i > 0 {
                // safe to unwrap: i is an index into the string, so it cannot
                // be any shorter.
                let literal = self.advance_by(i).unwrap();
                if !literal.is_empty() {
                    components.push(FormatComponent::Literal(literal.to_owned()));
                }
            }

            // safe to unwrap: we've only advanced as far as 'i', which is right
            // before the character it identified.
            let component = match self.advance_one().unwrap() {
                '\\' => self.parse_escape_sequence()?,
                '%' => self.parse_format_specifier()?,
                _ => panic!("{}", "Stopped at unexpected character: {self.string}"),
            };
            components.push(component);
        }

        if !self.string.is_empty() {
            components.push(FormatComponent::Literal(self.string.to_owned()));
        }

        Ok(FormatString { components })
    }
}

struct FormatString {
    components: Vec<FormatComponent>,
}

impl FormatString {
    fn parse(string: &str) -> Result<Self, Box<dyn Error>> {
        FormatStringParser { string }.parse()
    }
}

fn get_starting_point(file_info: &WalkEntry) -> &Path {
    file_info
        .path()
        .ancestors()
        .nth(file_info.depth())
        // safe to unwrap: the file's depth should never be longer than its path
        // (...right?).
        .unwrap()
}

fn format_non_link_file_type(file_type: FileType) -> char {
    match file_type {
        FileType::Regular => 'f',
        FileType::Directory => 'd',
        FileType::BlockDevice => 'b',
        FileType::CharDevice => 'c',
        FileType::Fifo => 'p',
        FileType::Socket => 's',
        _ => 'U',
    }
}

/// Apply a directive's precision and flags the way GNU find does: `%d` and
/// `%m` are printed like a C integer conversion (decimal and octal), `%S` is
/// left alone, and every other directive is a string, cut to the precision.
fn apply_precision_and_flags<'a>(
    directive: &FormatDirective,
    content: Cow<'a, str>,
    precision: Option<usize>,
    flags: &Flags,
    width: Option<usize>,
    justify: &Justify,
) -> Cow<'a, [u8]> {
    let octal = match directive {
        FormatDirective::Depth => false,
        FormatDirective::Permissions(PermissionsFormat::Octal) => true,
        FormatDirective::Sparseness => return str_bytes(content),
        _ => {
            // Like GNU find, the precision counts bytes, even inside a character.
            let mut bytes = str_bytes(content);
            if let Some(precision) = precision {
                match &mut bytes {
                    Cow::Borrowed(b) => *b = &b[..precision.min(b.len())],
                    Cow::Owned(b) => b.truncate(precision),
                }
            }
            return bytes;
        }
    };

    let mut digits = content.into_owned();
    if precision == Some(0) && digits == "0" {
        digits.clear();
    }
    if let Some(precision) = precision {
        let zeros = precision.saturating_sub(digits.len());
        digits.insert_str(0, &"0".repeat(zeros));
    }
    if octal && flags.alternate && !digits.starts_with('0') {
        digits.insert(0, '0');
    }
    let sign = match (octal, flags.plus, flags.space) {
        (false, true, _) => "+",
        (false, false, true) => " ",
        _ => "",
    };
    if let (Some(width), true) = (
        width,
        flags.zero && precision.is_none() && *justify == Justify::Right,
    ) {
        let pad = width.saturating_sub(sign.len());
        digits = format!("{digits:0>pad$}");
    }
    format!("{sign}{digits}").into_bytes().into()
}

fn str_bytes(s: Cow<'_, str>) -> Cow<'_, [u8]> {
    match s {
        Cow::Borrowed(s) => Cow::Borrowed(s.as_bytes()),
        Cow::Owned(s) => Cow::Owned(s.into_bytes()),
    }
}

fn format_directive<'entry>(
    file_info: &'entry WalkEntry,
    directive: &FormatDirective,
) -> Result<Cow<'entry, str>, Box<dyn Error>> {
    let meta = || file_info.metadata();

    // NOTE ON QUOTING:
    // GNU find's man page claims that several directives that print names (like
    // %f) are quoted like ls; however, I could not reproduce this at all in
    // practice, thus the set of rules is undoubtedly very different (if this is
    // still done at all).

    let res: Cow<'entry, str> = match directive {
        FormatDirective::AccessTime(tf) => tf.apply(meta()?.accessed()?)?,

        FormatDirective::Basename => file_info.file_name().to_string_lossy(),

        FormatDirective::Blocks { large_blocks } => {
            #[cfg(unix)]
            let len = meta()?.blocks() * STANDARD_BLOCK_SIZE;
            #[cfg(not(unix))]
            let len = meta()?.len();

            // GNU find says it returns the number of 512-byte blocks for %b,
            // but in reality it just returns the number of blocks, *regardless
            // of their size on the filesystem*. That behavior is copied here,
            // even though it's arguably not 100% correct.
            let bs = if *large_blocks { 1024 } else { 512 };
            let blocks = len.div_ceil(bs);

            blocks.to_string().into()
        }

        #[cfg(not(unix))]
        FormatDirective::ChangeTime(tf) => tf.apply(meta()?.modified()?)?,
        #[cfg(unix)]
        FormatDirective::ChangeTime(tf) => {
            use std::time::Duration;

            let meta = meta()?;
            let ctime = SystemTime::UNIX_EPOCH
                + Duration::from_secs(meta.ctime() as u64)
                + Duration::from_nanos(meta.ctime_nsec() as u64);
            tf.apply(ctime)?
        }

        FormatDirective::Depth => file_info.depth().to_string().into(),

        #[cfg(not(unix))]
        FormatDirective::Device => "0".into(),
        #[cfg(unix)]
        FormatDirective::Device => meta()?.dev().to_string().into(),

        // GNU find's behavior for this is a bit...odd:
        // - Both the root directory and the paths immediately underneath return an empty string
        // - Any path without any slashes (i.e. relative to cwd) returns "."
        // - "." also returns "."
        // - ".." returns "." (???)
        // These are all (thankfully) documented on the find(1) man page.
        FormatDirective::Dirname => match file_info.path().parent() {
            None => "".into(),
            Some(p) if p == Path::new("/") => "".into(),
            Some(p) if p == Path::new("") => ".".into(),
            Some(parent) => parent.to_string_lossy(),
        },

        #[cfg(not(unix))]
        FormatDirective::Filesystem => "".into(),
        #[cfg(unix)]
        FormatDirective::Filesystem => {
            let dev_id = meta()?.dev().to_string();
            let fs_list =
                uucore::fsext::read_fs_list().expect("Could not find the filesystem info");
            fs_list
                .into_iter()
                .rfind(|fs| fs.dev_id == dev_id)
                .map_or_else(String::new, |fs| fs.fs_type)
                .into()
        }

        #[cfg(not(unix))]
        FormatDirective::Group { .. } => "0".into(),
        #[cfg(unix)]
        FormatDirective::Group { as_name } => {
            let gid = meta()?.gid();
            if *as_name {
                uucore::entries::gid2grp(gid).unwrap_or_else(|_| gid.to_string())
            } else {
                gid.to_string()
            }
            .into()
        }

        #[cfg(not(unix))]
        FormatDirective::HardlinkCount => "0".into(),
        #[cfg(unix)]
        FormatDirective::HardlinkCount => meta()?.nlink().to_string().into(),

        #[cfg(not(unix))]
        FormatDirective::Inode => "0".into(),
        #[cfg(unix)]
        FormatDirective::Inode => meta()?.ino().to_string().into(),

        FormatDirective::ModificationTime(tf) => tf.apply(meta()?.modified()?)?,

        FormatDirective::Path {
            strip_starting_point,
        } => file_info
            .path()
            .strip_prefix(if *strip_starting_point {
                get_starting_point(file_info)
            } else {
                Path::new("")
            })
            // safe to unwrap: the prefix is derived *from* the path to begin
            // with, so it cannot be invalid.
            .unwrap()
            .to_string_lossy(),

        FormatDirective::Permissions(PermissionsFormat::Symbolic) => {
            uucore::fs::display_permissions(meta()?, true).into()
        }
        #[cfg(not(unix))]
        FormatDirective::Permissions(PermissionsFormat::Octal) => "777".into(),
        #[cfg(unix)]
        FormatDirective::Permissions(PermissionsFormat::Octal) => {
            format!("{:o}", meta()?.mode() & 0o7777).into()
        }

        FormatDirective::Size => meta()?.len().to_string().into(),

        #[cfg(not(unix))]
        FormatDirective::Sparseness => "1.0".into(),
        #[cfg(unix)]
        FormatDirective::Sparseness => {
            let meta = meta()?;

            if meta.len() > 0 {
                format!(
                    "{:.1}",
                    // GNU find hardcodes a block size of 512 bytes, regardless
                    // of the true filesystem block size.
                    (meta.blocks() * STANDARD_BLOCK_SIZE) as f64 / (meta.len() as f64)
                )
                .into()
            } else {
                "1.0".into()
            }
        }

        FormatDirective::StartingPoint => get_starting_point(file_info).to_string_lossy(),

        FormatDirective::SymlinkTarget => {
            if file_info.path_is_symlink() {
                fs::read_link(file_info.path())?
                    .to_string_lossy()
                    .into_owned()
                    .into()
            } else {
                "".into()
            }
        }

        FormatDirective::Type { follow_links } => if file_info.path_is_symlink() {
            if *follow_links {
                match file_info.path().metadata().map_err(WalkError::from) {
                    Ok(meta) => format_non_link_file_type(meta.file_type().into()),
                    Err(e) if e.is_not_found() => 'N',
                    Err(e) if e.is_loop() => 'L',
                    Err(_) => '?',
                }
            } else {
                'l'
            }
        } else {
            format_non_link_file_type(file_info.file_type())
        }
        .to_string()
        .into(),

        #[cfg(not(unix))]
        FormatDirective::User { .. } => "0".into(),
        #[cfg(unix)]
        FormatDirective::User { as_name } => {
            let uid = meta()?.uid();
            if *as_name {
                uucore::entries::uid2usr(uid).unwrap_or_else(|_| uid.to_string())
            } else {
                uid.to_string()
            }
            .into()
        }
    };

    Ok(res)
}

/// This matcher prints information about its files to stdout, following GNU
/// find's printf syntax.
pub struct Printf {
    format: FormatString,
    output_file: Option<(File, PathBuf)>,
    stdout_errors: StdoutErrors,
}

impl Printf {
    pub fn new(format: &str, output_file: Option<(File, PathBuf)>) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            format: FormatString::parse(format)?,
            output_file,
            stdout_errors: StdoutErrors::default(),
        })
    }

    pub fn with_stdout_errors(mut self, stdout_errors: StdoutErrors) -> Self {
        self.stdout_errors = stdout_errors;
        self
    }

    fn print(&self, file_info: &WalkEntry, mut out: impl Write) -> std::io::Result<()> {
        for component in &self.format.components {
            match component {
                FormatComponent::Literal(literal) => write!(out, "{literal}")?,
                FormatComponent::Byte(byte) => out.write_all(&[*byte])?,
                // \c stops the rest of this format, as in GNU find
                FormatComponent::Flush => {
                    out.flush()?;
                    break;
                }
                FormatComponent::Directive {
                    directive,
                    width,
                    precision,
                    justify,
                    flags,
                } => match format_directive(file_info, directive) {
                    Ok(content) => {
                        let content = apply_precision_and_flags(
                            directive, content, *precision, flags, *width, justify,
                        );
                        // GNU find pads to the width in bytes, not characters.
                        let pad = width.map_or(0, |width| width.saturating_sub(content.len()));
                        if *justify == Justify::Right {
                            write!(out, "{:pad$}", "")?;
                        }
                        out.write_all(&content)?;
                        if *justify == Justify::Left {
                            write!(out, "{:pad$}", "")?;
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "Error processing '{}': {}",
                            file_info.path().to_string_lossy(),
                            e
                        );
                        break;
                    }
                },
            }
        }

        Ok(())
    }

    fn handle_output_error(&self, error: &std::io::Error, matcher_io: &mut MatcherIO) {
        let Some((_, path)) = &self.output_file else {
            self.stdout_errors.handle(error, matcher_io);
            return;
        };
        if error.kind() == std::io::ErrorKind::BrokenPipe {
            matcher_io.quit();
            return;
        }
        let _ = writeln!(
            &mut stderr(),
            "{}: {}: {error}",
            crate::find::program_name(),
            path.display()
        );
        matcher_io.set_exit_code(1);
    }
}

impl Matcher for Printf {
    fn matches(&self, file_info: &WalkEntry, matcher_io: &mut MatcherIO) -> bool {
        let result = if let Some((file, _)) = &self.output_file {
            self.print(file_info, file)
        } else {
            self.print(file_info, &mut *matcher_io.deps.get_output().borrow_mut())
        };

        if let Err(error) = result {
            self.handle_output_error(&error, matcher_io);
        }

        true
    }

    fn has_side_effects(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::{ErrorKind, Write};

    use chrono::{Duration, TimeZone};
    use tempfile::Builder;

    use super::*;
    use crate::find::matchers::tests::get_dir_entry_for;
    use crate::find::tests::fix_up_slashes;
    use crate::find::tests::FakeDependencies;

    #[cfg(unix)]
    use std::os::unix::fs::{symlink, PermissionsExt};

    #[cfg(windows)]
    use std::os::windows::fs::{symlink_dir, symlink_file};

    struct FailingWriter(ErrorKind);

    impl Write for FailingWriter {
        fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
            Err(self.0.into())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Err(self.0.into())
        }
    }

    #[test]
    fn print_propagates_output_errors() -> Result<(), Box<dyn Error>> {
        let abbbc = get_dir_entry_for("./test_data/simple", "abbbc");

        for format in ["literal", "%p", "\\c", "\\377"] {
            let printf = Printf::new(format, None)?;
            assert_eq!(
                printf
                    .print(&abbbc, FailingWriter(ErrorKind::Other))
                    .map_err(|error| error.kind()),
                Err(ErrorKind::Other)
            );
        }

        Ok(())
    }

    #[test]
    fn handles_output_errors() -> Result<(), Box<dyn Error>> {
        let printf = Printf::new("literal", None)?;

        for (error_kind, initial_exit_code, expected_exit_code, expected_quit) in [
            (ErrorKind::Other, 0, 1, false),
            (ErrorKind::BrokenPipe, 0, 0, true),
            (ErrorKind::BrokenPipe, 1, 1, true),
        ] {
            let deps = FakeDependencies::new();
            let mut matcher_io = MatcherIO::new(&deps);
            matcher_io.set_exit_code(initial_exit_code);

            printf.handle_output_error(&error_kind.into(), &mut matcher_io);
            assert_eq!(matcher_io.exit_code(), expected_exit_code);
            assert_eq!(matcher_io.should_quit(), expected_quit);
        }

        Ok(())
    }

    #[test]
    fn test_parse_basics() {
        assert_eq!(FormatString::parse("").unwrap().components, vec![]);
        assert_eq!(
            FormatString::parse("test stuff").unwrap().components,
            vec![FormatComponent::Literal("test stuff".to_owned()),]
        );
    }

    #[test]
    fn test_parse_escapes() {
        assert_eq!(
            FormatString::parse("abc\\0\\t\\n\\\\\\141de\\cf")
                .unwrap()
                .components,
            vec![
                FormatComponent::Literal("abc".to_owned()),
                FormatComponent::Byte(0),
                FormatComponent::Literal("\t".to_owned()),
                FormatComponent::Literal("\n".to_owned()),
                FormatComponent::Literal("\\".to_owned()),
                FormatComponent::Byte(b'a'),
                FormatComponent::Literal("de".to_owned()),
                FormatComponent::Flush,
                FormatComponent::Literal("f".to_owned())
            ]
        );

        // An unrecognized escape is a warning, not an error: it's printed
        // literally (backslash included), matching GNU find.
        assert_eq!(
            FormatString::parse("\\X").unwrap().components,
            vec![FormatComponent::Literal("\\X".to_owned())]
        );
        assert_eq!(
            FormatString::parse("\\").unwrap().components,
            vec![FormatComponent::Literal("\\".to_owned())]
        );
        assert_eq!(
            FormatString::parse("abc\\").unwrap().components,
            vec![
                FormatComponent::Literal("abc".to_owned()),
                FormatComponent::Literal("\\".to_owned()),
            ]
        );
    }

    #[test]
    fn test_parse_octal_escape_before_multibyte_char() {
        assert_eq!(
            FormatString::parse("\\0€").unwrap().components,
            vec![
                FormatComponent::Byte(0),
                FormatComponent::Literal("€".to_owned()),
            ]
        );
        assert_eq!(
            FormatString::parse("\\1😀").unwrap().components,
            vec![
                FormatComponent::Byte(1),
                FormatComponent::Literal("😀".to_owned()),
            ]
        );
        assert_eq!(
            FormatString::parse("\\00é").unwrap().components,
            vec![
                FormatComponent::Byte(0),
                FormatComponent::Literal("é".to_owned()),
            ]
        );
    }

    #[test]
    fn test_parse_multibyte_char_after_directive() {
        assert_eq!(
            FormatString::parse("%€").unwrap().components,
            vec![FormatComponent::Literal("%€".to_owned())]
        );
        assert_eq!(
            FormatString::parse("\\€").unwrap().components,
            vec![FormatComponent::Literal("\\€".to_owned())]
        );
        assert!(FormatString::parse("%A€").is_err());
    }

    #[test]
    fn test_parse_formatting() {
        fn unaligned_directive(directive: FormatDirective) -> FormatComponent {
            FormatComponent::Directive {
                directive,
                width: None,
                precision: None,
                justify: Justify::Right,
                flags: Flags::default(),
            }
        }

        assert_eq!(
            FormatString::parse("%%%a%A@%Ak%b%c%C@%CH%d%DTEST%f%F%g%G%h%H")
                .unwrap()
                .components,
            vec![
                FormatComponent::Literal("%".to_owned()),
                unaligned_directive(FormatDirective::AccessTime(TimeFormat::Ctime)),
                unaligned_directive(FormatDirective::AccessTime(TimeFormat::SinceEpoch)),
                unaligned_directive(FormatDirective::AccessTime(TimeFormat::Strftime(
                    "%k".to_string()
                ))),
                unaligned_directive(FormatDirective::Blocks {
                    large_blocks: false
                }),
                unaligned_directive(FormatDirective::ChangeTime(TimeFormat::Ctime)),
                unaligned_directive(FormatDirective::ChangeTime(TimeFormat::SinceEpoch)),
                unaligned_directive(FormatDirective::ChangeTime(TimeFormat::Strftime(
                    "%H".to_string()
                ))),
                unaligned_directive(FormatDirective::Depth),
                unaligned_directive(FormatDirective::Device),
                FormatComponent::Literal("TEST".to_owned()),
                unaligned_directive(FormatDirective::Basename),
                unaligned_directive(FormatDirective::Filesystem),
                unaligned_directive(FormatDirective::Group { as_name: true }),
                unaligned_directive(FormatDirective::Group { as_name: false }),
                unaligned_directive(FormatDirective::Dirname),
                unaligned_directive(FormatDirective::StartingPoint),
            ]
        );

        assert_eq!(
            FormatString::parse("%i%k%l%m%M%n%p%P%s%S%t%T@%Td%u%U%y%Y%?")
                .unwrap()
                .components,
            vec![
                unaligned_directive(FormatDirective::Inode),
                unaligned_directive(FormatDirective::Blocks { large_blocks: true }),
                unaligned_directive(FormatDirective::SymlinkTarget),
                unaligned_directive(FormatDirective::Permissions(PermissionsFormat::Octal)),
                unaligned_directive(FormatDirective::Permissions(PermissionsFormat::Symbolic)),
                unaligned_directive(FormatDirective::HardlinkCount),
                unaligned_directive(FormatDirective::Path {
                    strip_starting_point: false
                }),
                unaligned_directive(FormatDirective::Path {
                    strip_starting_point: true
                }),
                unaligned_directive(FormatDirective::Size),
                unaligned_directive(FormatDirective::Sparseness),
                unaligned_directive(FormatDirective::ModificationTime(TimeFormat::Ctime)),
                unaligned_directive(FormatDirective::ModificationTime(TimeFormat::SinceEpoch)),
                unaligned_directive(FormatDirective::ModificationTime(TimeFormat::Strftime(
                    "%d".to_string()
                ))),
                unaligned_directive(FormatDirective::User { as_name: true }),
                unaligned_directive(FormatDirective::User { as_name: false }),
                unaligned_directive(FormatDirective::Type {
                    follow_links: false
                }),
                unaligned_directive(FormatDirective::Type { follow_links: true }),
                // An unrecognized directive is a warning, not a silent drop of
                // the '%': it's printed literally, matching GNU find.
                FormatComponent::Literal("%?".to_owned()),
            ]
        );

        assert!(FormatString::parse("%").is_err());
        assert!(FormatString::parse("%A!").is_err());
    }

    #[test]
    fn test_parse_formatting_justified() {
        assert_eq!(
            FormatString::parse("%d%-s%5S%-12n% 3f% -- 4i")
                .unwrap()
                .components,
            vec![
                FormatComponent::Directive {
                    directive: FormatDirective::Depth,
                    justify: Justify::Right,
                    width: None,
                    precision: None,
                    flags: Flags::default(),
                },
                FormatComponent::Directive {
                    directive: FormatDirective::Size,
                    justify: Justify::Left,
                    width: None,
                    precision: None,
                    flags: Flags::default(),
                },
                FormatComponent::Directive {
                    directive: FormatDirective::Sparseness,
                    justify: Justify::Right,
                    width: Some(5),
                    precision: None,
                    flags: Flags::default(),
                },
                FormatComponent::Directive {
                    directive: FormatDirective::HardlinkCount,
                    justify: Justify::Left,
                    width: Some(12),
                    precision: None,
                    flags: Flags::default(),
                },
                FormatComponent::Directive {
                    directive: FormatDirective::Basename,
                    justify: Justify::Right,
                    width: Some(3),
                    precision: None,
                    flags: Flags {
                        space: true,
                        ..Flags::default()
                    },
                },
                FormatComponent::Directive {
                    directive: FormatDirective::Inode,
                    justify: Justify::Left,
                    width: Some(4),
                    precision: None,
                    flags: Flags {
                        space: true,
                        ..Flags::default()
                    },
                },
            ]
        );
    }

    #[test]
    fn test_parse_precision_and_flags() {
        assert_eq!(
            FormatString::parse("%-+ #08.3d").unwrap().components,
            vec![FormatComponent::Directive {
                directive: FormatDirective::Depth,
                width: Some(8),
                precision: Some(3),
                justify: Justify::Left,
                flags: Flags {
                    plus: true,
                    space: true,
                    alternate: true,
                    zero: true,
                },
            }]
        );
        assert_eq!(
            FormatString::parse("%.f").unwrap().components,
            vec![FormatComponent::Directive {
                directive: FormatDirective::Basename,
                width: None,
                precision: Some(0),
                justify: Justify::Right,
                flags: Flags::default(),
            }]
        );
    }

    #[test]
    fn test_printf_precision_and_flags() {
        let file_info = get_dir_entry_for("test_data/simple", "abbbc");
        let deps = FakeDependencies::new();

        let matcher =
            Printf::new("%.3f|%6.2f|%05f|%+f|%+d|%+04d|% d|%.2d|%.0d|%-4d|", None).unwrap();
        assert!(matcher.matches(&file_info, &mut deps.new_matcher_io()));
        assert_eq!(
            "abb|    ab|abbbc|abbbc|+1|+001| 1|01|1|1   |",
            deps.get_output_as_string()
        );
    }

    #[test]
    fn test_printf_precision_big_and_depth_zero() {
        let file_info = get_dir_entry_for("test_data/simple", "abbbc");
        let deps = FakeDependencies::new();
        let matcher = Printf::new("%.70000f|", None).unwrap();
        assert!(matcher.matches(&file_info, &mut deps.new_matcher_io()));
        assert_eq!("abbbc|", deps.get_output_as_string());

        let file_info = get_dir_entry_for("test_data/simple", "");
        let deps = FakeDependencies::new();
        let matcher = Printf::new("%.0d|%.1d|%#d|", None).unwrap();
        assert!(matcher.matches(&file_info, &mut deps.new_matcher_io()));
        assert_eq!("|0|0|", deps.get_output_as_string());
    }

    #[test]
    fn test_printf_unrecognized_directive_keeps_its_text() {
        let file_info = get_dir_entry_for("test_data/simple", "abbbc");
        let deps = FakeDependencies::new();

        // GNU find only takes `0` after the other flags, so `%0-5d` is not `%d`.
        let matcher = Printf::new("%0-5d|%+.2z|", None).unwrap();
        assert!(matcher.matches(&file_info, &mut deps.new_matcher_io()));
        assert_eq!("%0-5d|%+.2z|", deps.get_output_as_string());
    }

    #[test]
    fn test_printf_precision_and_width_count_bytes() {
        let temp_dir = Builder::new().prefix("example").tempdir().unwrap();
        let temp_dir_path = temp_dir.path().to_string_lossy();
        File::create(temp_dir.path().join("\u{e9}_ab")).expect("create temp file");
        let file_info = get_dir_entry_for(&temp_dir_path, "\u{e9}_ab");

        let matcher = Printf::new("%.3f|%5.2f|%-5.1f|", None).unwrap();
        let mut out = Vec::new();
        matcher.print(&file_info, &mut out).unwrap();
        assert_eq!(b"\xc3\xa9_|   \xc3\xa9|\xc3    |", out.as_slice());
    }

    #[test]
    #[cfg(unix)]
    fn test_printf_permissions_precision_and_flags() {
        let temp_dir = Builder::new().prefix("example").tempdir().unwrap();
        let temp_dir_path = temp_dir.path().to_string_lossy();
        let matcher = Printf::new("%#m|%05m|%#.0m|%#5m|%#05m|%.0m|", None).unwrap();

        for (mode, expected) in [
            (0o644, "0644|00644|0644| 0644|00644|644|"),
            (0o000, "0|00000|0|    0|00000||"),
            (0o4755, "04755|04755|04755|04755|04755|4755|"),
        ] {
            let name = format!("mode{mode:o}");
            let file = File::create(temp_dir.path().join(&name)).expect("create temp file");
            let mut perms = file.metadata().unwrap().permissions();
            perms.set_mode(mode);
            file.set_permissions(perms).unwrap();

            let file_info = get_dir_entry_for(&temp_dir_path, &name);
            let deps = FakeDependencies::new();
            assert!(matcher.matches(&file_info, &mut deps.new_matcher_io()));
            assert_eq!(expected, deps.get_output_as_string());
        }
    }

    #[test]
    fn test_printf_justified() {
        let file_info = get_dir_entry_for("test_data/simple", "abbbc");
        let deps = FakeDependencies::new();

        let matcher = Printf::new("%f,%7f,%-7f", None).unwrap();
        assert!(matcher.matches(&file_info, &mut deps.new_matcher_io()));
        assert_eq!("abbbc,  abbbc,abbbc  ", deps.get_output_as_string());
    }

    #[test]
    fn test_printf_backslash_c_stops_the_format() {
        let file_info = get_dir_entry_for("test_data/simple", "abbbc");
        let deps = FakeDependencies::new();

        let matcher = Printf::new("%f\\c%s\n", None).unwrap();
        assert!(matcher.matches(&file_info, &mut deps.new_matcher_io()));
        assert_eq!("abbbc", deps.get_output_as_string());
    }

    #[test]
    fn test_printf_paths() {
        let file_info = get_dir_entry_for("test_data/simple", "abbbc");
        let deps = FakeDependencies::new();

        let matcher = Printf::new("%h %H %p %P", None).unwrap();
        assert!(matcher.matches(&file_info, &mut deps.new_matcher_io()));
        assert_eq!(
            format!(
                "{} {} {} {}",
                fix_up_slashes("test_data/simple"),
                fix_up_slashes("test_data/simple"),
                fix_up_slashes("test_data/simple/abbbc"),
                fix_up_slashes("abbbc")
            ),
            deps.get_output_as_string()
        );
    }

    #[test]
    fn test_printf_paths_in_subdir() {
        let file_info = get_dir_entry_for("test_data/simple", "subdir/ABBBC");
        let deps = FakeDependencies::new();

        let matcher = Printf::new("%h %H %p %P", None).unwrap();
        assert!(matcher.matches(&file_info, &mut deps.new_matcher_io()));
        assert_eq!(
            format!(
                "{} {} {} {}",
                fix_up_slashes("test_data/simple/subdir"),
                fix_up_slashes("test_data/simple"),
                fix_up_slashes("test_data/simple/subdir/ABBBC"),
                fix_up_slashes("subdir/ABBBC")
            ),
            deps.get_output_as_string()
        );
    }

    #[test]
    fn test_printf_depth() {
        let file_info_1 = get_dir_entry_for("test_data/depth/1", "f1");
        let file_info_2 = get_dir_entry_for("test_data/depth/1", "2/f2");
        let deps = FakeDependencies::new();

        let matcher = Printf::new("%d.", None).unwrap();
        assert!(matcher.matches(&file_info_1, &mut deps.new_matcher_io()));
        assert!(matcher.matches(&file_info_2, &mut deps.new_matcher_io()));
        assert_eq!("1.2.", deps.get_output_as_string());
    }

    #[test]
    fn test_printf_basic_types() {
        let file_info_f = get_dir_entry_for("test_data/simple", "abbbc");
        let file_info_d = get_dir_entry_for("test_data/simple", "subdir");
        let deps = FakeDependencies::new();

        let matcher = Printf::new("%y", None).unwrap();
        assert!(matcher.matches(&file_info_f, &mut deps.new_matcher_io()));
        assert!(matcher.matches(&file_info_d, &mut deps.new_matcher_io()));
        assert_eq!("fd", deps.get_output_as_string());
    }

    #[test]
    #[cfg(unix)]
    fn test_printf_special_types() {
        use std::os::unix::net::UnixListener;

        use nix::sys::stat::Mode;

        let temp_dir = Builder::new().prefix("example").tempdir().unwrap();
        let temp_dir_path = temp_dir.path().to_string_lossy();

        let fifo_name = "fifo";
        let fifo_path = temp_dir.path().join(fifo_name);
        nix::unistd::mkfifo(&fifo_path, Mode::from_bits(0o644).unwrap()).unwrap();

        let socket_name = "socket";
        let socket_path = temp_dir.path().join(socket_name);
        UnixListener::bind(socket_path).unwrap();

        let fifo_info = get_dir_entry_for(&temp_dir_path, fifo_name);
        let socket_info = get_dir_entry_for(&temp_dir_path, socket_name);
        let deps = FakeDependencies::new();

        let matcher = Printf::new("%y", None).unwrap();
        assert!(matcher.matches(&fifo_info, &mut deps.new_matcher_io()));
        assert!(matcher.matches(&socket_info, &mut deps.new_matcher_io()));
        assert_eq!("ps", deps.get_output_as_string());
    }

    #[test]
    fn test_printf_size() {
        let file_info = get_dir_entry_for("test_data/size", "512bytes");
        let deps = FakeDependencies::new();

        let matcher = Printf::new("%s", None).unwrap();
        assert!(matcher.matches(&file_info, &mut deps.new_matcher_io()));
        assert_eq!("512", deps.get_output_as_string());
    }

    #[test]
    fn test_printf_symlinks() {
        #[cfg(unix)]
        {
            if let Err(e) = symlink("abbbc", "test_data/links/link-f") {
                assert!(
                    e.kind() == ErrorKind::AlreadyExists,
                    "Failed to create sym link: {e:?}"
                );
            }
            if let Err(e) = symlink("subdir", "test_data/links/link-d") {
                assert!(
                    e.kind() == ErrorKind::AlreadyExists,
                    "Failed to create sym link: {e:?}"
                );
            }
            if let Err(e) = symlink("missing", "test_data/links/link-missing") {
                assert!(
                    e.kind() == ErrorKind::AlreadyExists,
                    "Failed to create sym link: {e:?}"
                );
            }
            if let Err(e) = symlink("abbbc/x", "test_data/links/link-notdir") {
                assert!(
                    e.kind() == ErrorKind::AlreadyExists,
                    "Failed to create sym link: {e:?}"
                );
            }
            if let Err(e) = symlink("link-loop", "test_data/links/link-loop") {
                assert!(
                    e.kind() == ErrorKind::AlreadyExists,
                    "Failed to create sym link: {e:?}"
                );
            }
        }
        #[cfg(windows)]
        {
            if let Err(e) = symlink_file("abbbc", "test_data/links/link-f") {
                assert!(
                    e.kind() == ErrorKind::AlreadyExists,
                    "Failed to create sym link: {:?}",
                    e
                );
            }
            if let Err(e) = symlink_dir("subdir", "test_data/links/link-d") {
                assert!(
                    e.kind() == ErrorKind::AlreadyExists,
                    "Failed to create sym link: {:?}",
                    e
                );
            }
            if let Err(e) = symlink_file("missing", "test_data/links/link-missing") {
                assert!(
                    e.kind() == ErrorKind::AlreadyExists,
                    "Failed to create sym link: {:?}",
                    e
                );
            }
            if let Err(e) = symlink_file("abbbc/x", "test_data/links/link-notdir") {
                assert!(
                    e.kind() == ErrorKind::AlreadyExists,
                    "Failed to create sym link: {:?}",
                    e
                );
            }
        }

        let regular_file = get_dir_entry_for("test_data/simple", "abbbc");
        let link_f = get_dir_entry_for("test_data/links", "link-f");
        let link_d = get_dir_entry_for("test_data/links", "link-d");
        let link_missing = get_dir_entry_for("test_data/links", "link-missing");
        let link_notdir = get_dir_entry_for("test_data/links", "link-notdir");
        #[cfg(unix)]
        let link_loop = get_dir_entry_for("test_data/links", "link-loop");

        let deps = FakeDependencies::new();

        let matcher = Printf::new("%y %Y %l\n", None).unwrap();
        assert!(matcher.matches(&regular_file, &mut deps.new_matcher_io()));
        assert!(matcher.matches(&link_f, &mut deps.new_matcher_io()));
        assert!(matcher.matches(&link_d, &mut deps.new_matcher_io()));
        assert!(matcher.matches(&link_missing, &mut deps.new_matcher_io()));
        assert!(matcher.matches(&link_notdir, &mut deps.new_matcher_io()));
        #[cfg(unix)]
        assert!(matcher.matches(&link_loop, &mut deps.new_matcher_io()));
        assert_eq!(
            vec![
                "f f ",
                "l f abbbc",
                "l d subdir",
                "l N missing",
                // We can't detect ENOTDIR on non-unix platforms yet.
                #[cfg(not(unix))]
                "l ? abbbc/x",
                #[cfg(unix)]
                "l N abbbc/x",
                #[cfg(unix)]
                "l L link-loop",
            ],
            deps.get_output_as_string().lines().collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_printf_times() {
        let temp_dir = Builder::new().prefix("example").tempdir().unwrap();
        let temp_dir_path = temp_dir.path().to_string_lossy();
        let new_file_name = "newFile";
        let file_path = temp_dir.path().join(new_file_name);
        File::create(&file_path).expect("create temp file");

        let mtime = chrono::Local
            .with_ymd_and_hms(2000, 1, 15, 9, 30, 21)
            .unwrap()
            + Duration::nanoseconds(2_000_000);
        filetime::set_file_mtime(
            &file_path,
            filetime::FileTime::from_unix_time(mtime.timestamp(), mtime.timestamp_subsec_nanos()),
        )
        .expect("set temp file mtime");

        let file_info = get_dir_entry_for(&temp_dir_path, new_file_name);
        let deps = FakeDependencies::new();

        let matcher = Printf::new("%t,%T@,%TF", None).unwrap();
        assert!(matcher.matches(&file_info, &mut deps.new_matcher_io()));
        assert_eq!(
            format!(
                "Sat Jan 15 09:30:21.0020000000 2000,{}.0020000000,2000-01-15",
                mtime.timestamp()
            ),
            deps.get_output_as_string()
        );
    }

    #[test]
    fn test_printf_ctime_pads_day_with_space() {
        let temp_dir = Builder::new().prefix("example").tempdir().unwrap();
        let temp_dir_path = temp_dir.path().to_string_lossy();
        let new_file_name = "newFile";
        let file_path = temp_dir.path().join(new_file_name);
        File::create(&file_path).expect("create temp file");

        let mtime = chrono::Local
            .with_ymd_and_hms(2000, 1, 2, 9, 30, 21)
            .unwrap();
        filetime::set_file_mtime(
            &file_path,
            filetime::FileTime::from_unix_time(mtime.timestamp(), 0),
        )
        .expect("set temp file mtime");

        let file_info = get_dir_entry_for(&temp_dir_path, new_file_name);
        let deps = FakeDependencies::new();

        // like ctime(3): "Jan  2", not "Jan 02"
        let matcher = Printf::new("%t", None).unwrap();
        assert!(matcher.matches(&file_info, &mut deps.new_matcher_io()));
        assert_eq!(
            "Sun Jan  2 09:30:21.0000000000 2000",
            deps.get_output_as_string()
        );
    }

    #[test]
    fn test_printf_time_plus_format() {
        let temp_dir = Builder::new().prefix("example").tempdir().unwrap();
        let temp_dir_path = temp_dir.path().to_string_lossy();
        let new_file_name = "newFile";
        let file_path = temp_dir.path().join(new_file_name);
        File::create(&file_path).expect("create temp file");

        // GNU findutils always prints a dot and ten fractional digits, even
        // for timestamps that fall on a whole second.
        for (nanos, expected_time) in [
            (0, "2000-01-15+09:30:21.0000000000"),
            (500_000_000, "2000-01-15+09:30:21.5000000000"),
        ] {
            let mtime = chrono::Local
                .with_ymd_and_hms(2000, 1, 15, 9, 30, 21)
                .unwrap()
                + Duration::nanoseconds(nanos);
            filetime::set_file_mtime(
                &file_path,
                filetime::FileTime::from_unix_time(
                    mtime.timestamp(),
                    mtime.timestamp_subsec_nanos(),
                ),
            )
            .expect("set temp file mtime");

            let file_info = get_dir_entry_for(&temp_dir_path, new_file_name);
            let deps = FakeDependencies::new();

            let matcher = Printf::new("%T+", None).unwrap();
            assert!(matcher.matches(&file_info, &mut deps.new_matcher_io()));
            assert_eq!(expected_time, deps.get_output_as_string());
        }
    }

    #[test]
    #[cfg(unix)]
    fn test_printf_user_group() {
        let temp_dir = Builder::new().prefix("example").tempdir().unwrap();
        let temp_dir_path = temp_dir.path().to_string_lossy();
        let new_file_name = "newFile";
        File::create(temp_dir.path().join(new_file_name)).expect("create temp file");

        let uid = unsafe { uucore::libc::getuid() };
        let user = uucore::entries::uid2usr(uid).unwrap_or(uid.to_string());

        let gid = unsafe { uucore::libc::getgid() };
        let group = uucore::entries::gid2grp(gid).unwrap_or(gid.to_string());

        let file_info = get_dir_entry_for(&temp_dir_path, new_file_name);
        let deps = FakeDependencies::new();

        let matcher = Printf::new("%u %U %g %G", None).unwrap();
        assert!(matcher.matches(&file_info, &mut deps.new_matcher_io()));
        assert_eq!(
            format!("{user} {uid} {group} {gid}"),
            deps.get_output_as_string()
        );
    }

    #[test]
    #[cfg(unix)]
    fn test_printf_permissions() {
        use std::fs::File;

        let temp_dir = Builder::new().prefix("example").tempdir().unwrap();
        let temp_dir_path = temp_dir.path().to_string_lossy();

        for (mode, expected) in [
            (0o755, "755 -rwxr-xr-x"),
            (0o4755, "4755 -rwsr-xr-x"),
            (0o044, "44 ----r--r--"),
        ] {
            let new_file_name = format!("file{mode:o}");
            let file = File::create(temp_dir.path().join(&new_file_name)).unwrap();
            let mut perms = file.metadata().unwrap().permissions();
            perms.set_mode(mode);
            file.set_permissions(perms).unwrap();

            let file_info = get_dir_entry_for(&temp_dir_path, &new_file_name);
            let deps = FakeDependencies::new();

            let matcher = Printf::new("%m %M", None).unwrap();
            assert!(matcher.matches(&file_info, &mut deps.new_matcher_io()));
            assert_eq!(expected, deps.get_output_as_string());
        }
    }
}
