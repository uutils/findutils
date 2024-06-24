// Copyright 2017 Google Inc.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

mod access;
mod delete;
mod empty;
pub mod exec;
mod glob;
mod group;
mod lname;
mod logical_matchers;
mod name;
mod path;
mod perm;
mod printer;
mod printf;
mod prune;
mod quit;
mod regex;
mod size;
#[cfg(unix)]
mod stat;
mod time;
mod type_matcher;
mod user;

use ::regex::Regex;
use chrono::{DateTime, Datelike, NaiveDateTime, Utc};
use std::path::Path;
use std::time::SystemTime;
use std::{error::Error, str::FromStr};
use walkdir::DirEntry;

use self::access::AccessMatcher;
use self::delete::DeleteMatcher;
use self::empty::EmptyMatcher;
use self::exec::SingleExecMatcher;
use self::group::{GroupMatcher, NoGroupMatcher};
use self::lname::LinkNameMatcher;
use self::logical_matchers::{
    AndMatcherBuilder, FalseMatcher, ListMatcherBuilder, NotMatcher, TrueMatcher,
};
use self::name::NameMatcher;
use self::path::PathMatcher;
use self::perm::PermMatcher;
use self::printer::{PrintDelimiter, Printer};
use self::printf::Printf;
use self::prune::PruneMatcher;
use self::quit::QuitMatcher;
use self::regex::RegexMatcher;
use self::size::SizeMatcher;
#[cfg(unix)]
use self::stat::{InodeMatcher, LinksMatcher};
use self::time::{
    FileAgeRangeMatcher, FileTimeMatcher, FileTimeType, NewerMatcher, NewerOptionMatcher,
    NewerOptionType, NewerTimeMatcher,
};
use self::type_matcher::TypeMatcher;
use self::user::{NoUserMatcher, UserMatcher};

use super::{Config, Dependencies};

/// Struct holding references to outputs and any inputs that can't be derived
/// from the file/directory info.
pub struct MatcherIO<'a> {
    should_skip_dir: bool,
    quit: bool,
    deps: &'a dyn Dependencies<'a>,
}

impl<'a> MatcherIO<'a> {
    pub fn new(deps: &'a dyn Dependencies<'a>) -> MatcherIO<'a> {
        MatcherIO {
            deps,
            should_skip_dir: false,
            quit: false,
        }
    }

    pub fn mark_current_dir_to_be_skipped(&mut self) {
        self.should_skip_dir = true;
    }

    #[must_use]
    pub fn should_skip_current_dir(&self) -> bool {
        self.should_skip_dir
    }

    pub fn quit(&mut self) {
        self.quit = true;
    }

    #[must_use]
    pub fn should_quit(&self) -> bool {
        self.quit
    }

    #[must_use]
    pub fn now(&self) -> SystemTime {
        self.deps.now()
    }
}

/// A basic interface that can be used to determine whether a directory entry
/// is what's being searched for. To a first order approximation, find consists
/// of building a chain of Matcher objects, and then walking a directory tree,
/// passing each entry to the chain of Matchers.
pub trait Matcher: 'static {
    /// Boxes this matcher as a trait object.
    fn into_box(self) -> Box<dyn Matcher>
    where
        Self: Sized,
    {
        Box::new(self)
    }

    /// Returns whether the given file matches the object's predicate.
    fn matches(&self, file_info: &DirEntry, matcher_io: &mut MatcherIO) -> bool;

    /// Returns whether the matcher has any side-effects (e.g. executing a
    /// command, deleting a file). Iff no such matcher exists in the chain, then
    /// the filename will be printed to stdout. While this is a compile-time
    /// fact for most matchers, it's run-time for matchers that contain a
    /// collection of sub-Matchers.
    fn has_side_effects(&self) -> bool {
        // most matchers don't have side-effects, so supply a default implementation.
        false
    }

    /// Notification that find has finished processing a given directory.
    fn finished_dir(&self, _finished_directory: &Path) {}

    /// Notification that find has finished processing all directories -
    /// allowing for any cleanup that isn't suitable for destructors (e.g.
    /// blocking calls, I/O etc.)
    fn finished(&self) {}
}

impl Matcher for Box<dyn Matcher> {
    fn into_box(self) -> Box<dyn Matcher> {
        self
    }

    fn matches(&self, file_info: &DirEntry, matcher_io: &mut MatcherIO) -> bool {
        (**self).matches(file_info, matcher_io)
    }

    fn has_side_effects(&self) -> bool {
        (**self).has_side_effects()
    }

    fn finished_dir(&self, finished_directory: &Path) {
        (**self).finished_dir(finished_directory);
    }

    fn finished(&self) {
        (**self).finished();
    }
}

pub enum ComparableValue {
    MoreThan(u64),
    EqualTo(u64),
    LessThan(u64),
}

impl ComparableValue {
    fn matches(&self, value: u64) -> bool {
        match *self {
            ComparableValue::MoreThan(limit) => value > limit,
            ComparableValue::EqualTo(limit) => value == limit,
            ComparableValue::LessThan(limit) => value < limit,
        }
    }

    /// same as matches, but takes a signed value
    fn imatches(&self, value: i64) -> bool {
        match *self {
            ComparableValue::MoreThan(limit) => value >= 0 && (value as u64) > limit,
            ComparableValue::EqualTo(limit) => value >= 0 && (value as u64) == limit,
            ComparableValue::LessThan(limit) => value < 0 || (value as u64) < limit,
        }
    }
}

/// Builds a single `AndMatcher` containing the Matcher objects corresponding
/// to the passed in predicate arguments.
pub fn build_top_level_matcher(
    args: &[&str],
    config: &mut Config,
) -> Result<Box<dyn Matcher>, Box<dyn Error>> {
    let (_, top_level_matcher) = (build_matcher_tree(args, config, 0, false))?;

    // if the matcher doesn't have any side-effects, then we default to printing
    if !top_level_matcher.has_side_effects() {
        let mut new_and_matcher = AndMatcherBuilder::new();
        new_and_matcher.new_and_condition(top_level_matcher);
        new_and_matcher.new_and_condition(Printer::new(PrintDelimiter::Newline));
        return Ok(new_and_matcher.build());
    }
    Ok(top_level_matcher)
}

/// Helper function for `build_matcher_tree`.
fn are_more_expressions(args: &[&str], index: usize) -> bool {
    (index < args.len() - 1) && args[index + 1] != ")"
}

fn convert_arg_to_number(
    option_name: &str,
    value_as_string: &str,
) -> Result<usize, Box<dyn Error>> {
    match value_as_string.parse::<usize>() {
        Ok(val) => Ok(val),
        _ => Err(From::from(format!(
            "Expected a positive decimal integer argument to {option_name}, but got \
             `{value_as_string}'"
        ))),
    }
}

fn convert_arg_to_comparable_value(
    option_name: &str,
    value_as_string: &str,
) -> Result<ComparableValue, Box<dyn Error>> {
    let re = Regex::new(r"^([+-]?)(\d+)$")?;
    if let Some(groups) = re.captures(value_as_string) {
        if let Ok(val) = groups[2].parse::<u64>() {
            return Ok(match &groups[1] {
                "+" => ComparableValue::MoreThan(val),
                "-" => ComparableValue::LessThan(val),
                _ => ComparableValue::EqualTo(val),
            });
        }
    }
    Err(From::from(format!(
        "Expected a decimal integer (with optional + or - prefix) argument \
         to {option_name}, but got `{value_as_string}'"
    )))
}

fn convert_arg_to_comparable_value_and_suffix(
    option_name: &str,
    value_as_string: &str,
) -> Result<(ComparableValue, String), Box<dyn Error>> {
    let re = Regex::new(r"([+-]?)(\d+)(.*)$")?;
    if let Some(groups) = re.captures(value_as_string) {
        if let Ok(val) = groups[2].parse::<u64>() {
            return Ok((
                match &groups[1] {
                    "+" => ComparableValue::MoreThan(val),
                    "-" => ComparableValue::LessThan(val),
                    _ => ComparableValue::EqualTo(val),
                },
                groups[3].to_string(),
            ));
        }
    }
    Err(From::from(format!(
        "Expected a decimal integer (with optional + or - prefix) and \
         (optional suffix) argument to {option_name}, but got `{value_as_string}'"
    )))
}

/// This is a function that converts a specific string format into a timestamp.
/// It allows converting a time string of
/// "(week abbreviation) (date), (year) (time)" to a Unix timestamp.
/// such as: "jan 01, 2025 00:00:01" -> 1735689601000
/// When (time) is not provided, it will be automatically filled in as 00:00:00
/// such as: "jan 01, 2025" = "jan 01, 2025 00:00:00" -> 1735689600000
fn parse_date_str_to_timestamps(date_str: &str) -> Option<i64> {
    let regex_pattern =
        r"^(?P<month_day>\w{3} \d{2})?(?:, (?P<year>\d{4}))?(?: (?P<time>\d{2}:\d{2}:\d{2}))?$";
    let re = Regex::new(regex_pattern);

    if let Some(captures) = re.ok()?.captures(date_str) {
        let now = Utc::now();
        let month_day = captures
            .get(1)
            .map_or(format!("{} {}", now.format("%b"), now.format("%d")), |m| {
                m.as_str().to_string()
            });
        // If no year input.
        let year = captures
            .get(2)
            .map_or(now.year(), |m| m.as_str().parse().unwrap());
        // If the user does not enter a specific time, it will be filled with 0
        let time_str = captures.get(3).map_or("00:00:00", |m| m.as_str());
        let date_time_str = format!("{month_day}, {year} {time_str}");
        let datetime = NaiveDateTime::parse_from_str(&date_time_str, "%b %d, %Y %H:%M:%S").ok()?;
        let utc_datetime = DateTime::<Utc>::from_naive_utc_and_offset(datetime, Utc);
        Some(utc_datetime.timestamp_millis())
    } else {
        None
    }
}

/// This function implements the function of matching substrings of
/// X and Y from the -newerXY string.
/// X and Y are constrained to a/B/c/m and t.
/// such as: "-neweraB" -> Some(a, B) "-neweraD" -> None
///
/// Additionally, there is support for the -anewer and -cnewer short arguments. as follows:
/// 1. -anewer is equivalent to -neweram
/// 2. -cnewer is equivalent to - newercm
///
/// If -newer is used it will be resolved to -newermm.
fn parse_str_to_newer_args(input: &str) -> Option<(String, String)> {
    if input.is_empty() {
        return None;
    }

    if input == "-newer" {
        return Some(("m".to_string(), "m".to_string()));
    }

    if input == "-anewer" {
        return Some(("a".to_string(), "m".to_string()));
    }

    if input == "-cnewer" {
        return Some(("c".to_string(), "m".to_string()));
    }

    let re = Regex::new(r"-newer([aBcm])([aBcmt])").unwrap();
    if let Some(captures) = re.captures(input) {
        let x = captures.get(1)?.as_str().to_string();
        let y = captures.get(2)?.as_str().to_string();
        Some((x, y))
    } else {
        None
    }
}

/// The main "translate command-line args into a matcher" function. Will call
/// itself recursively if it encounters an opening bracket. A successful return
/// consists of a tuple containing the new index into the args array to use (if
/// called recursively) and the resulting matcher.
fn build_matcher_tree(
    args: &[&str],
    config: &mut Config,
    arg_index: usize,
    expecting_bracket: bool,
) -> Result<(usize, Box<dyn Matcher>), Box<dyn Error>> {
    let mut top_level_matcher = ListMatcherBuilder::new();

    let mut regex_type = regex::RegexType::default();

    // can't use getopts for a variety or reasons:
    // order of arguments is important
    // arguments can start with + as well as -
    // multiple-character flags don't start with a double dash
    let mut i = arg_index;
    let mut invert_next_matcher = false;
    while i < args.len() {
        let possible_submatcher = match args[i] {
            "-print" => Some(Printer::new(PrintDelimiter::Newline).into_box()),
            "-print0" => Some(Printer::new(PrintDelimiter::Null).into_box()),
            "-printf" => {
                if i >= args.len() - 1 {
                    return Err(From::from(format!("missing argument to {}", args[i])));
                }
                i += 1;
                Some(Printf::new(args[i])?.into_box())
            }
            "-true" => Some(TrueMatcher.into_box()),
            "-false" => Some(FalseMatcher.into_box()),
            "-lname" | "-ilname" => {
                if i >= args.len() - 1 {
                    return Err(From::from(format!("missing argument to {}", args[i])));
                }
                i += 1;
                Some(LinkNameMatcher::new(args[i], args[i - 1].starts_with("-i")).into_box())
            }
            "-name" | "-iname" => {
                if i >= args.len() - 1 {
                    return Err(From::from(format!("missing argument to {}", args[i])));
                }
                i += 1;
                Some(NameMatcher::new(args[i], args[i - 1].starts_with("-i")).into_box())
            }
            "-path" | "-ipath" | "-wholename" | "-iwholename" => {
                if i >= args.len() - 1 {
                    return Err(From::from(format!("missing argument to {}", args[i])));
                }
                i += 1;
                Some(PathMatcher::new(args[i], args[i - 1].starts_with("-i")).into_box())
            }
            "-readable" => Some(AccessMatcher::Readable.into_box()),
            "-regextype" => {
                if i >= args.len() - 1 {
                    return Err(From::from(format!("missing argument to {}", args[i])));
                }
                i += 1;
                regex_type = regex::RegexType::from_str(args[i])?;
                None
            }
            "-regex" => {
                if i >= args.len() - 1 {
                    return Err(From::from(format!("missing argument to {}", args[i])));
                }
                i += 1;
                Some(RegexMatcher::new(regex_type, args[i], false)?.into_box())
            }
            "-iregex" => {
                if i >= args.len() - 1 {
                    return Err(From::from(format!("missing argument to {}", args[i])));
                }
                i += 1;
                Some(RegexMatcher::new(regex_type, args[i], true)?.into_box())
            }
            "-type" => {
                if i >= args.len() - 1 {
                    return Err(From::from(format!("missing argument to {}", args[i])));
                }
                i += 1;
                Some(TypeMatcher::new(args[i])?.into_box())
            }
            "-delete" => {
                // -delete implicitly requires -depth
                config.depth_first = true;
                Some(DeleteMatcher::new().into_box())
            }
            "-newer" => {
                if i >= args.len() - 1 {
                    return Err(From::from(format!("missing argument to {}", args[i])));
                }
                i += 1;
                Some(NewerMatcher::new(args[i])?.into_box())
            }
            "-mtime" | "-atime" | "-ctime" => {
                if i >= args.len() - 1 {
                    return Err(From::from(format!("missing argument to {}", args[i])));
                }
                let file_time_type = match args[i] {
                    "-atime" => FileTimeType::Accessed,
                    "-ctime" => FileTimeType::Created,
                    "-mtime" => FileTimeType::Modified,
                    // This shouldn't be possible. We've already checked the value
                    // is one of those three values.
                    _ => unreachable!("Encountered unexpected value {}", args[i]),
                };
                let days = convert_arg_to_comparable_value(args[i], args[i + 1])?;
                i += 1;
                Some(FileTimeMatcher::new(file_time_type, days).into_box())
            }
            "-amin" | "-cmin" | "-mmin" => {
                if i >= args.len() - 1 {
                    return Err(From::from(format!("missing argument to {}", args[i])));
                }
                let file_time_type = match args[i] {
                    "-amin" => FileTimeType::Accessed,
                    "-cmin" => FileTimeType::Created,
                    "-mmin" => FileTimeType::Modified,
                    _ => unreachable!("Encountered unexpected value {}", args[i]),
                };
                let minutes = convert_arg_to_comparable_value(args[i], args[i + 1])?;
                i += 1;
                Some(FileAgeRangeMatcher::new(file_time_type, minutes).into_box())
            }
            "-size" => {
                if i >= args.len() - 1 {
                    return Err(From::from(format!("missing argument to {}", args[i])));
                }
                let (size, unit) =
                    convert_arg_to_comparable_value_and_suffix(args[i], args[i + 1])?;
                i += 1;
                Some(SizeMatcher::new(size, &unit)?.into_box())
            }
            "-empty" => Some(EmptyMatcher::new().into_box()),
            "-exec" | "-execdir" => {
                let mut arg_index = i + 1;
                while arg_index < args.len() && args[arg_index] != ";" {
                    if args[arg_index - 1] == "{}" && args[arg_index] == "+" {
                        // MultiExecMatcher isn't written yet
                        return Err(From::from(format!(
                            "{} [args...] + isn't supported yet. \
                             Only {} [args...] ;",
                            args[i], args[i]
                        )));
                    }
                    arg_index += 1;
                }
                if arg_index < i + 2 || arg_index == args.len() {
                    // at the minimum we need the executable and the ';'
                    return Err(From::from(format!("missing argument to {}", args[i])));
                }
                let expression = args[i];
                let executable = args[i + 1];
                let exec_args = &args[i + 2..arg_index];
                i = arg_index;
                Some(
                    SingleExecMatcher::new(executable, exec_args, expression == "-execdir")?
                        .into_box(),
                )
            }
            #[cfg(unix)]
            "-inum" => {
                if i >= args.len() - 1 {
                    return Err(From::from(format!("missing argument to {}", args[i])));
                }
                let inum = convert_arg_to_comparable_value(args[i], args[i + 1])?;
                i += 1;
                Some(InodeMatcher::new(inum).into_box())
            }
            #[cfg(not(unix))]
            "-inum" => {
                return Err(From::from(
                    "Inode numbers are not available on this platform",
                ));
            }
            #[cfg(unix)]
            "-links" => {
                if i >= args.len() - 1 {
                    return Err(From::from(format!("missing argument to {}", args[i])));
                }
                let inum = convert_arg_to_comparable_value(args[i], args[i + 1])?;
                i += 1;
                Some(LinksMatcher::new(inum).into_box())
            }
            #[cfg(not(unix))]
            "-links" => {
                return Err(From::from("Link counts are not available on this platform"));
            }
            "-user" => {
                if i >= args.len() - 1 {
                    return Err(From::from(format!("missing argument to {}", args[i])));
                }

                let user = args[i + 1];

                if user.is_empty() {
                    return Err(From::from("The argument to -user should not be empty"));
                }

                i += 1;
                let matcher = UserMatcher::new(user.to_string());
                match matcher.uid() {
                    Some(_) => Some(matcher.into_box()),
                    None => {
                        return Err(From::from(format!(
                            "{} is not the name of a known user",
                            user
                        )))
                    }
                }
            }
            "-nouser" => Some(NoUserMatcher {}.into_box()),
            "-group" => {
                if i >= args.len() - 1 {
                    return Err(From::from(format!("missing argument to {}", args[i])));
                }

                let group = args[i + 1];

                if group.is_empty() {
                    return Err(From::from(
                        "Argument to -group is empty, but should be a group name",
                    ));
                }

                i += 1;
                let matcher = GroupMatcher::new(group.to_string());
                match matcher.gid() {
                    Some(_) => Some(matcher.into_box()),
                    None => {
                        return Err(From::from(format!(
                            "{} is not the name of an existing group",
                            group
                        )))
                    }
                }
            }
            "-nogroup" => Some(NoGroupMatcher {}.into_box()),
            "-executable" => Some(AccessMatcher::Executable.into_box()),
            "-perm" => {
                if i >= args.len() - 1 {
                    return Err(From::from(format!("missing argument to {}", args[i])));
                }
                i += 1;
                Some(PermMatcher::new(args[i])?.into_box())
            }
            "-prune" => Some(PruneMatcher::new().into_box()),
            "-quit" => Some(QuitMatcher.into_box()),
            "-writable" => Some(AccessMatcher::Writable.into_box()),
            "-not" | "!" => {
                if !are_more_expressions(args, i) {
                    return Err(From::from(format!(
                        "expected an expression after {}",
                        args[i]
                    )));
                }
                invert_next_matcher = !invert_next_matcher;
                None
            }
            "-and" | "-a" => {
                if !are_more_expressions(args, i) {
                    return Err(From::from(format!(
                        "expected an expression after {}",
                        args[i]
                    )));
                }
                top_level_matcher.check_new_and_condition()?;
                None
            }
            "-or" | "-o" => {
                if !are_more_expressions(args, i) {
                    return Err(From::from(format!(
                        "expected an expression after {}",
                        args[i]
                    )));
                }
                top_level_matcher.new_or_condition(args[i])?;
                None
            }
            "," => {
                if !are_more_expressions(args, i) {
                    return Err(From::from(format!(
                        "expected an expression after {}",
                        args[i]
                    )));
                }
                top_level_matcher.new_list_condition()?;
                None
            }
            "(" => {
                let (new_arg_index, sub_matcher) = build_matcher_tree(args, config, i + 1, true)?;
                i = new_arg_index;
                Some(sub_matcher)
            }
            ")" => {
                if !expecting_bracket {
                    return Err(From::from("you have too many ')'"));
                }

                let bracket = args[i - 1];
                if bracket == "(" {
                    return Err(From::from(
                        "invalid expression; empty parentheses are not allowed.",
                    ));
                }

                return Ok((i, top_level_matcher.build()));
            }
            "-d" | "-depth" => {
                // TODO add warning if it appears after actual testing criterion
                config.depth_first = true;
                None
            }
            "-mount" | "-xdev" => {
                // TODO add warning if it appears after actual testing criterion
                config.same_file_system = true;
                None
            }
            "-sorted" => {
                // TODO add warning if it appears after actual testing criterion
                config.sorted_output = true;
                None
            }
            "-maxdepth" => {
                if i >= args.len() - 1 {
                    return Err(From::from(format!("missing argument to {}", args[i])));
                }
                config.max_depth = convert_arg_to_number(args[i], args[i + 1])?;
                i += 1;
                None
            }
            "-mindepth" => {
                if i >= args.len() - 1 {
                    return Err(From::from(format!("missing argument to {}", args[i])));
                }
                config.min_depth = convert_arg_to_number(args[i], args[i + 1])?;
                i += 1;
                None
            }
            "-help" | "--help" => {
                config.help_requested = true;
                None
            }
            "-version" | "--version" => {
                config.version_requested = true;
                None
            }

            _ => {
                match parse_str_to_newer_args(args[i]) {
                    Some((x_option, y_option)) => {
                        if i >= args.len() - 1 {
                            return Err(From::from(format!("missing argument to {}", args[i])));
                        }
                        #[cfg(target_os = "linux")]
                        if x_option == "B" {
                            return Err(From::from("find: This system does not provide a way to find the birth time of a file."));
                        }
                        if y_option == "t" {
                            let time = args[i + 1];
                            let newer_time_type = NewerOptionType::from_str(x_option.as_str());
                            // Convert args to unix timestamps. (expressed in numeric types)
                            let comparable_time = match parse_date_str_to_timestamps(time) {
                                Some(timestamp) => timestamp,
                                None => {
                                    return Err(From::from(format!(
                                        "find: I cannot figure out how to interpret ‘{}’ as a date or time",
                                        args[i + 1]
                                    )))
                                }
                            };
                            i += 1;
                            Some(NewerTimeMatcher::new(newer_time_type, comparable_time).into_box())
                        } else {
                            let file_path = args[i + 1];
                            i += 1;
                            Some(NewerOptionMatcher::new(x_option, y_option, file_path)?.into_box())
                        }
                    }
                    None => return Err(From::from(format!("Unrecognized flag: '{}'", args[i]))),
                }
            }
        };
        if let Some(submatcher) = possible_submatcher {
            if invert_next_matcher {
                top_level_matcher.new_and_condition(NotMatcher::new(submatcher));
                invert_next_matcher = false;
            } else {
                top_level_matcher.new_and_condition(submatcher);
            }
        }
        i += 1;
    }
    if expecting_bracket {
        return Err(From::from(
            "invalid expression; I was expecting to find a ')' somewhere but \
             did not see one.",
        ));
    }
    Ok((i, top_level_matcher.build()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::find::tests::fix_up_slashes;
    use crate::find::tests::FakeDependencies;
    use walkdir::WalkDir;

    /// Helper function for tests to get a `DirEntry` object. directory should
    /// probably be a string starting with `test_data/` (cargo's tests run with
    /// a working directory set to the root findutils folder).
    pub fn get_dir_entry_for(directory: &str, filename: &str) -> DirEntry {
        for wrapped_dir_entry in WalkDir::new(fix_up_slashes(directory)) {
            let dir_entry = wrapped_dir_entry.unwrap();
            if dir_entry
                .path()
                .strip_prefix(directory)
                .unwrap()
                .to_string_lossy()
                == fix_up_slashes(filename)
            {
                return dir_entry;
            }
        }
        panic!("Couldn't find {filename} in {directory}");
    }

    #[test]
    fn build_top_level_matcher_name() {
        let abbbc_lower = get_dir_entry_for("./test_data/simple", "abbbc");
        let abbbc_upper = get_dir_entry_for("./test_data/simple/subdir", "ABBBC");
        let mut config = Config::default();
        let deps = FakeDependencies::new();

        let matcher = build_top_level_matcher(&["-name", "a*c"], &mut config).unwrap();

        assert!(matcher.matches(&abbbc_lower, &mut deps.new_matcher_io()));
        assert!(!matcher.matches(&abbbc_upper, &mut deps.new_matcher_io()));
        assert_eq!(
            deps.get_output_as_string(),
            fix_up_slashes("./test_data/simple/abbbc\n")
        );
    }

    #[test]
    fn build_top_level_matcher_iname() {
        let abbbc_lower = get_dir_entry_for("./test_data/simple", "abbbc");
        let abbbc_upper = get_dir_entry_for("./test_data/simple/subdir", "ABBBC");
        let mut config = Config::default();
        let deps = FakeDependencies::new();

        let matcher = build_top_level_matcher(&["-iname", "a*c"], &mut config).unwrap();

        assert!(matcher.matches(&abbbc_lower, &mut deps.new_matcher_io()));
        assert!(matcher.matches(&abbbc_upper, &mut deps.new_matcher_io()));
        assert_eq!(
            deps.get_output_as_string(),
            fix_up_slashes("./test_data/simple/abbbc\n./test_data/simple/subdir/ABBBC\n")
        );
    }

    #[test]
    fn build_top_level_matcher_not() {
        for arg in &["-not", "!"] {
            let abbbc_lower = get_dir_entry_for("./test_data/simple", "abbbc");
            let mut config = Config::default();
            let deps = FakeDependencies::new();

            let matcher =
                build_top_level_matcher(&[arg, "-name", "does_not_exist"], &mut config).unwrap();

            assert!(matcher.matches(&abbbc_lower, &mut deps.new_matcher_io()));
            assert_eq!(
                deps.get_output_as_string(),
                fix_up_slashes("./test_data/simple/abbbc\n")
            );
        }
    }

    #[test]
    fn build_top_level_matcher_not_needs_expression() {
        for arg in &["-not", "!"] {
            let mut config = Config::default();

            if let Err(e) = build_top_level_matcher(&[arg], &mut config) {
                assert!(e.to_string().contains("expected an expression"));
            } else {
                panic!("parsing argument lists that end in -not should fail");
            }
        }
    }

    #[test]
    fn build_top_level_matcher_not_double_negation() {
        for arg in &["-not", "!"] {
            let abbbc_lower = get_dir_entry_for("./test_data/simple", "abbbc");
            let mut config = Config::default();
            let deps = FakeDependencies::new();

            let matcher =
                build_top_level_matcher(&[arg, arg, "-name", "abbbc"], &mut config).unwrap();

            assert!(matcher.matches(&abbbc_lower, &mut deps.new_matcher_io()));
            assert_eq!(
                deps.get_output_as_string(),
                fix_up_slashes("./test_data/simple/abbbc\n")
            );

            config = Config::default();
            let matcher =
                build_top_level_matcher(&[arg, arg, "-name", "does_not_exist"], &mut config)
                    .unwrap();

            assert!(!matcher.matches(&abbbc_lower, &mut deps.new_matcher_io()));
        }
    }

    #[test]
    fn build_top_level_matcher_missing_args() {
        for arg in &["-iname", "-name", "-type"] {
            let mut config = Config::default();

            if let Err(e) = build_top_level_matcher(&[arg], &mut config) {
                assert!(e.to_string().contains("missing argument to"));
                assert!(e.to_string().contains(arg));
            } else {
                panic!("parsing argument lists that end in -not should fail");
            }
        }
    }

    #[test]
    fn build_top_level_matcher_or_without_expr1() {
        for arg in &["-or", "-o"] {
            let mut config = Config::default();

            if let Err(e) = build_top_level_matcher(&[arg, "-true"], &mut config) {
                assert!(e.to_string().contains("you have used a binary operator"));
            } else {
                panic!("parsing argument list that begins with -or should fail");
            }
        }
    }

    #[test]
    fn build_top_level_matcher_or_without_expr2() {
        for arg in &["-or", "-o"] {
            let mut config = Config::default();

            if let Err(e) = build_top_level_matcher(&["-true", arg], &mut config) {
                assert!(e.to_string().contains("expected an expression"));
            } else {
                panic!("parsing argument list that ends with -or should fail");
            }
        }
    }

    #[test]
    fn build_top_level_matcher_and_without_expr1() {
        let mut config = Config::default();

        if let Err(e) = build_top_level_matcher(&["-a", "-true"], &mut config) {
            assert!(e.to_string().contains("you have used a binary operator"));
        } else {
            panic!("parsing argument list that begins with -a should fail");
        }
    }

    #[test]
    fn build_top_level_matcher_and_without_expr2() {
        let mut config = Config::default();

        if let Err(e) = build_top_level_matcher(&["-true", "-a"], &mut config) {
            assert!(e.to_string().contains("expected an expression"));
        } else {
            panic!("parsing argument list that ends with -or should fail");
        }
    }

    #[test]
    fn build_top_level_matcher_dash_a_works() {
        for arg in &["-a", "-and"] {
            let abbbc = get_dir_entry_for("./test_data/simple", "abbbc");
            let mut config = Config::default();
            let deps = FakeDependencies::new();

            // build a matcher using an explicit -a argument
            let matcher = build_top_level_matcher(&["-true", arg, "-true"], &mut config).unwrap();
            assert!(matcher.matches(&abbbc, &mut deps.new_matcher_io()));
            assert_eq!(
                deps.get_output_as_string(),
                fix_up_slashes("./test_data/simple/abbbc\n")
            );
        }
    }

    #[test]
    fn build_top_level_matcher_or_works() {
        let abbbc = get_dir_entry_for("./test_data/simple", "abbbc");
        for args in &[
            ["-true", "-o", "-false"],
            ["-false", "-o", "-true"],
            ["-true", "-o", "-true"],
        ] {
            let mut config = Config::default();
            let deps = FakeDependencies::new();

            let matcher = build_top_level_matcher(args, &mut config).unwrap();

            assert!(matcher.matches(&abbbc, &mut deps.new_matcher_io()));
            assert_eq!(
                deps.get_output_as_string(),
                fix_up_slashes("./test_data/simple/abbbc\n")
            );
        }

        let mut config = Config::default();
        let deps = FakeDependencies::new();

        let matcher = build_top_level_matcher(&["-false", "-o", "-false"], &mut config).unwrap();

        assert!(!matcher.matches(&abbbc, &mut deps.new_matcher_io()));
        assert_eq!(deps.get_output_as_string(), "");
    }

    #[test]
    fn build_top_level_matcher_and_works() {
        let abbbc = get_dir_entry_for("./test_data/simple", "abbbc");
        for args in &[
            ["-true", "-false"],
            ["-false", "-true"],
            ["-false", "-false"],
        ] {
            let mut config = Config::default();
            let deps = FakeDependencies::new();

            let matcher = build_top_level_matcher(args, &mut config).unwrap();

            assert!(!matcher.matches(&abbbc, &mut deps.new_matcher_io()));
            assert_eq!(deps.get_output_as_string(), "");
        }

        let mut config = Config::default();
        let deps = FakeDependencies::new();

        let matcher = build_top_level_matcher(&["-true", "-true"], &mut config).unwrap();

        assert!(matcher.matches(&abbbc, &mut deps.new_matcher_io()));
        assert_eq!(
            deps.get_output_as_string(),
            fix_up_slashes("./test_data/simple/abbbc\n")
        );
    }

    #[test]
    fn build_top_level_matcher_list_works() {
        let abbbc = get_dir_entry_for("./test_data/simple", "abbbc");
        let args = ["-true", "-print", "-false", ",", "-print", "-false"];
        let mut config = Config::default();
        let deps = FakeDependencies::new();

        let matcher = build_top_level_matcher(&args, &mut config).unwrap();

        // final matcher returns false, so list matcher should too
        assert!(!matcher.matches(&abbbc, &mut deps.new_matcher_io()));
        // two print matchers means doubled output
        assert_eq!(
            deps.get_output_as_string(),
            fix_up_slashes("./test_data/simple/abbbc\n./test_data/simple/abbbc\n")
        );
    }

    #[test]
    fn build_top_level_matcher_list_without_expr1() {
        let mut config = Config::default();

        if let Err(e) = build_top_level_matcher(&[",", "-true"], &mut config) {
            assert!(e.to_string().contains("you have used a binary operator"));
        } else {
            panic!("parsing argument list that begins with , should fail");
        }

        if let Err(e) = build_top_level_matcher(&["-true", "-o", ",", "-true"], &mut config) {
            assert!(e.to_string().contains("you have used a binary operator"));
        } else {
            panic!("parsing argument list that contains '-o  ,' should fail");
        }
    }

    #[test]
    fn build_top_level_matcher_list_without_expr2() {
        let mut config = Config::default();

        if let Err(e) = build_top_level_matcher(&["-true", ","], &mut config) {
            assert!(e.to_string().contains("expected an expression"));
        } else {
            panic!("parsing argument list that ends with , should fail");
        }
    }

    #[test]
    fn build_top_level_matcher_not_enough_brackets() {
        let mut config = Config::default();

        if let Err(e) = build_top_level_matcher(&["-true", "("], &mut config) {
            assert!(e.to_string().contains("I was expecting to find a ')'"));
        } else {
            panic!("parsing argument list with not enough closing brackets should fail");
        }
    }

    #[test]
    fn build_top_level_matcher_too_many_brackets() {
        let mut config = Config::default();

        if let Err(e) = build_top_level_matcher(
            &["-type", "f", "(", "-name", "*.txt", ")", ")"],
            &mut config,
        ) {
            assert!(e.to_string().contains("too many ')'"));
        } else {
            panic!("parsing argument list with too many closing brackets should fail");
        }
    }

    #[test]
    fn build_top_level_matcher_can_use_bracket_as_arg() {
        let mut config = Config::default();
        // make sure that if we use a bracket as an argument (e.g. to -name)
        // then it isn't viewed as a bracket
        build_top_level_matcher(&["-name", "("], &mut config).unwrap();
        build_top_level_matcher(&["-name", ")"], &mut config).unwrap();
    }

    #[test]
    fn build_top_level_matcher_brackets_work() {
        let abbbc = get_dir_entry_for("./test_data/simple", "abbbc");
        // same as true | ( false & false) = true
        let args_without = ["-true", "-o", "-false", "-false"];
        // same as (true | false) & false = false
        let args_with = ["(", "-true", "-o", "-false", ")", "-false"];
        let mut config = Config::default();
        let deps = FakeDependencies::new();

        {
            let matcher = build_top_level_matcher(&args_without, &mut config).unwrap();
            assert!(matcher.matches(&abbbc, &mut deps.new_matcher_io()));
        }
        {
            let matcher = build_top_level_matcher(&args_with, &mut config).unwrap();
            assert!(!matcher.matches(&abbbc, &mut deps.new_matcher_io()));
        }
    }

    #[test]
    fn build_top_level_matcher_not_and_brackets_work() {
        let abbbc = get_dir_entry_for("./test_data/simple", "abbbc");
        // same as (true & !(false)) | true = true
        let args_without = ["-true", "-not", "-false", "-o", "-true"];
        // same as true & !(false | true) = false
        let args_with = ["-true", "-not", "(", "-false", "-o", "-true", ")"];
        let mut config = Config::default();
        let deps = FakeDependencies::new();

        {
            let matcher = build_top_level_matcher(&args_without, &mut config).unwrap();
            assert!(matcher.matches(&abbbc, &mut deps.new_matcher_io()));
        }
        {
            let matcher = build_top_level_matcher(&args_with, &mut config).unwrap();
            assert!(!matcher.matches(&abbbc, &mut deps.new_matcher_io()));
        }
    }

    #[test]
    fn build_top_level_matcher_expression_empty_parentheses() {
        let mut config = Config::default();

        if let Err(e) = build_top_level_matcher(&["-true", "(", ")"], &mut config) {
            assert!(e.to_string().contains("empty parentheses are not allowed"));
        } else {
            panic!("parsing argument list with empty parentheses in an expression should fail");
        }
    }

    #[test]
    fn comparable_value_matches() {
        assert!(
            !ComparableValue::LessThan(0).matches(0),
            "0 should not be less than 0"
        );
        assert!(
            ComparableValue::LessThan(u64::MAX).matches(0),
            "0 should be less than max_value"
        );
        assert!(
            !ComparableValue::LessThan(0).matches(u64::MAX),
            "max_value should not be less than 0"
        );
        assert!(
            !ComparableValue::LessThan(u64::MAX).matches(u64::MAX),
            "max_value should not be less than max_value"
        );

        assert!(
            ComparableValue::EqualTo(0).matches(0),
            "0 should be equal to 0"
        );
        assert!(
            !ComparableValue::EqualTo(u64::MAX).matches(0),
            "0 should not be equal to max_value"
        );
        assert!(
            !ComparableValue::EqualTo(0).matches(u64::MAX),
            "max_value should not be equal to 0"
        );
        assert!(
            ComparableValue::EqualTo(u64::MAX).matches(u64::MAX),
            "max_value should be equal to max_value"
        );

        assert!(
            !ComparableValue::MoreThan(0).matches(0),
            "0 should not be more than 0"
        );
        assert!(
            !ComparableValue::MoreThan(u64::MAX).matches(0),
            "0 should not be more than max_value"
        );
        assert!(
            ComparableValue::MoreThan(0).matches(u64::MAX),
            "max_value should be more than 0"
        );
        assert!(
            !ComparableValue::MoreThan(u64::MAX).matches(u64::MAX),
            "max_value should not be more than max_value"
        );
    }

    #[test]
    fn comparable_value_imatches() {
        assert!(
            !ComparableValue::LessThan(0).imatches(0),
            "0 should not be less than 0"
        );
        assert!(
            ComparableValue::LessThan(u64::MAX).imatches(0),
            "0 should be less than max_value"
        );
        assert!(
            !ComparableValue::LessThan(0).imatches(i64::MAX),
            "max_value should not be less than 0"
        );
        assert!(
            ComparableValue::LessThan(u64::MAX).imatches(i64::MAX),
            "max_value should be less than max_value"
        );
        assert!(
            ComparableValue::LessThan(0).imatches(i64::MIN),
            "min_value should be less than 0"
        );
        assert!(
            ComparableValue::LessThan(u64::MAX).imatches(i64::MIN),
            "min_value should be less than max_value"
        );

        assert!(
            ComparableValue::EqualTo(0).imatches(0),
            "0 should be equal to 0"
        );
        assert!(
            !ComparableValue::EqualTo(u64::MAX).imatches(0),
            "0 should not be equal to max_value"
        );
        assert!(
            !ComparableValue::EqualTo(0).imatches(i64::MAX),
            "max_value should not be equal to 0"
        );
        assert!(
            !ComparableValue::EqualTo(u64::MAX).imatches(i64::MAX),
            "max_value should not be equal to i64::max_value"
        );
        assert!(
            ComparableValue::EqualTo(i64::MAX as u64).imatches(i64::MAX),
            "i64::max_value should be equal to i64::max_value"
        );
        assert!(
            !ComparableValue::EqualTo(0).imatches(i64::MIN),
            "min_value should not be equal to 0"
        );
        assert!(
            !ComparableValue::EqualTo(u64::MAX).imatches(i64::MIN),
            "min_value should not be equal to max_value"
        );

        assert!(
            !ComparableValue::MoreThan(0).imatches(0),
            "0 should not be more than 0"
        );
        assert!(
            !ComparableValue::MoreThan(u64::MAX).imatches(0),
            "0 should not be more than max_value"
        );
        assert!(
            ComparableValue::MoreThan(0).imatches(i64::MAX),
            "max_value should be more than 0"
        );
        assert!(
            !ComparableValue::MoreThan(u64::MAX).imatches(i64::MAX),
            "max_value should not be more than max_value"
        );
        assert!(
            !ComparableValue::MoreThan(0).imatches(i64::MIN),
            "min_value should not be more than 0"
        );
        assert!(
            !ComparableValue::MoreThan(u64::MAX).imatches(i64::MIN),
            "min_value should not be more than max_value"
        );
    }

    #[test]
    fn build_top_level_matcher_bad_ctime_value() {
        let mut config = Config::default();

        if let Err(e) = build_top_level_matcher(&["-ctime", "-123."], &mut config) {
            assert!(
                e.to_string().contains("Expected a decimal integer"),
                "bad description: {e}"
            );
        } else {
            panic!("parsing a bad ctime value should fail");
        }
    }

    #[test]
    fn build_top_level_exec_not_enough_args() {
        let mut config = Config::default();

        if let Err(e) = build_top_level_matcher(&["-exec"], &mut config) {
            assert!(e.to_string().contains("missing argument"));
        } else {
            panic!("parsing argument list with exec and no executable or semi-colon should fail");
        }

        if let Err(e) = build_top_level_matcher(&["-exec", ";"], &mut config) {
            assert!(e.to_string().contains("missing argument"));
        } else {
            panic!("parsing argument list with exec and no executable should fail");
        }

        if let Err(e) = build_top_level_matcher(&["-exec", "foo"], &mut config) {
            assert!(e.to_string().contains("missing argument"));
        } else {
            panic!("parsing argument list with exec and no executable should fail");
        }
    }

    #[test]
    fn build_top_level_exec_should_eat_args() {
        let mut config = Config::default();
        build_top_level_matcher(&["-exec", "foo", "-o", "(", ";"], &mut config)
            .expect("parsing argument list with exec that takes brackets and -os should work");
    }

    #[test]
    fn build_top_level_exec_plus_semicolon() {
        let mut config = Config::default();
        build_top_level_matcher(&["-exec", "foo", "{}", "foo", "+", ";"], &mut config)
            .expect("only {} + should be considered a multi-exec");
    }

    #[test]
    #[cfg(unix)]
    fn build_top_level_matcher_perm() {
        let abbbc = get_dir_entry_for("./test_data/simple", "abbbc");
        let mut config = Config::default();

        // this should match: abbbc is readable
        let matcher_readable = build_top_level_matcher(&["-perm", "-u+r"], &mut config).unwrap();
        // this shouldn't match: abbbc isn't executable
        let matcher_executable = build_top_level_matcher(&["-perm", "-u+x"], &mut config).unwrap();

        let deps = FakeDependencies::new();
        assert!(matcher_readable.matches(&abbbc, &mut deps.new_matcher_io()));
        assert_eq!(deps.get_output_as_string(), "./test_data/simple/abbbc\n");

        let deps = FakeDependencies::new();
        assert!(!matcher_executable.matches(&abbbc, &mut deps.new_matcher_io()));
        assert_eq!(deps.get_output_as_string(), "");
    }

    #[test]
    #[cfg(unix)]
    fn build_top_level_matcher_perm_bad() {
        let mut config = Config::default();
        if let Err(e) = build_top_level_matcher(&["-perm", "foo"], &mut config) {
            assert!(e.to_string().contains("invalid operator"));
        } else {
            panic!("-perm with bad mode pattern should fail");
        }

        if let Err(e) = build_top_level_matcher(&["-perm"], &mut config) {
            assert!(e.to_string().contains("missing argument"));
        } else {
            panic!("-perm with no mode pattern should fail");
        }
    }

    #[test]
    #[cfg(not(unix))]
    fn build_top_level_matcher_perm_not_unix() {
        let mut config = Config::default();
        if let Err(e) = build_top_level_matcher(&["-perm", "444"], &mut config) {
            assert!(e.to_string().contains("not available"));
        } else {
            panic!("-perm on non-unix systems shouldn't be available");
        }

        if let Err(e) = build_top_level_matcher(&["-perm"], &mut config) {
            assert!(e.to_string().contains("missing argument"));
        } else {
            panic!("-perm with no mode pattern should fail");
        }
    }

    #[test]
    fn convert_exception_arg_to_comparable_value_test() {
        let exception_args = ["1%2", "1%2%3", "1a2", "1%2a", "abc", "-", "+", "%"];

        for arg in exception_args {
            let comparable = convert_arg_to_comparable_value("test", arg);
            assert!(
                comparable.is_err(),
                "{} should be parse to Comparable correctly",
                arg
            );
        }
    }

    #[test]
    fn parse_date_str_to_timestamps_test() {
        let full_date_timestamps = parse_date_str_to_timestamps("jan 01, 2025 00:00:01").unwrap();
        assert!(full_date_timestamps.to_string().contains("1735689601000"));

        let not_include_time_date_timestamps =
            parse_date_str_to_timestamps("jan 01, 2025").unwrap();
        assert!(not_include_time_date_timestamps
            .to_string()
            .contains("1735689600000"));

        // pass if return current time.
        let none_date_timestamps = parse_date_str_to_timestamps("");
        let now_but_zero_hour_min_sec = Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp_millis();
        assert_eq!(none_date_timestamps, Some(now_but_zero_hour_min_sec));
    }

    #[test]
    fn parse_str_to_newer_args_test() {
        // test for error case
        let arg = parse_str_to_newer_args("");
        assert!(arg.is_none());

        // test for short options
        // -newer equivalent to -newermm
        let arg = parse_str_to_newer_args("-newer").unwrap();
        assert_eq!(("m".to_string(), "m".to_string()), arg);

        // -anewer equivalent to -neweram
        let arg = parse_str_to_newer_args("-anewer").unwrap();
        assert_eq!(("a".to_string(), "m".to_string()), arg);

        // -cnewer equivalent to - newercm
        let arg = parse_str_to_newer_args("-cnewer").unwrap();
        assert_eq!(("c".to_string(), "m".to_string()), arg);

        let x_options = ["a", "B", "c", "m"];
        let y_options = ["a", "B", "c", "m", "t"];

        for &x in x_options.iter() {
            for &y in &y_options {
                let eq: (String, String) = (String::from(x), String::from(y));
                let arg = parse_str_to_newer_args(&format!("-newer{x}{y}").to_string()).unwrap();
                assert_eq!(eq, arg);
            }
        }
    }
}
