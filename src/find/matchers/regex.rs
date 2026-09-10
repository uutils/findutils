// Copyright 2022 Collabora, Ltd.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::{
    error::Error,
    fmt,
    io::{stderr, Write},
    str::FromStr,
};

use super::regex_transpile;

use super::{Matcher, MatcherIO, WalkEntry};

#[derive(Debug)]
pub struct ParseRegexTypeError(String);

impl Error for ParseRegexTypeError {}

impl fmt::Display for ParseRegexTypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Invalid regex type: {} (must be one of {})",
            self.0,
            RegexType::VALUES
                .iter()
                .map(|t| format!("'{t}'"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum RegexType {
    #[default]
    Emacs,
    Grep,
    PosixBasic,
    PosixExtended,
}

impl RegexType {
    pub const VALUES: &'static [Self] = &[
        Self::Emacs,
        Self::Grep,
        Self::PosixBasic,
        Self::PosixExtended,
    ];
}

impl fmt::Display for RegexType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Emacs => write!(f, "emacs"),
            Self::Grep => write!(f, "grep"),
            Self::PosixBasic => write!(f, "posix-basic"),
            Self::PosixExtended => write!(f, "posix-extended"),
        }
    }
}

impl FromStr for RegexType {
    type Err = ParseRegexTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "emacs" => Ok(Self::Emacs),
            "grep" => Ok(Self::Grep),
            "posix-basic" => Ok(Self::PosixBasic),
            "posix-extended" => Ok(Self::PosixExtended),
            // ed and sed are the same as posix-basic
            "ed" | "sed" => Ok(Self::PosixBasic),
            _ => Err(ParseRegexTypeError(s.to_owned())),
        }
    }
}

pub struct RegexMatcher {
    regex: fancy_regex::Regex,
}

impl RegexMatcher {
    pub fn new(
        regex_type: RegexType,
        pattern: &str,
        ignore_case: bool,
    ) -> Result<Self, Box<dyn Error>> {
        // GNU find's -regex does full-path matching, so anchor the pattern.
        let anchored = match regex_type {
            RegexType::PosixExtended => format!("^(?:{pattern})$"),
            RegexType::Emacs | RegexType::PosixBasic | RegexType::Grep => {
                format!(r"^\({pattern}\)$")
            }
        };
        Ok(Self {
            regex: regex_transpile::compile(&anchored, regex_type, ignore_case)?,
        })
    }
}

impl Matcher for RegexMatcher {
    fn matches(&self, file_info: &WalkEntry, matcher_io: &mut MatcherIO) -> bool {
        match self
            .regex
            .is_match(file_info.path().to_string_lossy().as_ref())
        {
            Ok(matched) => matched,
            Err(e) => {
                let _ = writeln!(&mut stderr(), "find: {e}");
                matcher_io.set_exit_code(1);
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::find::matchers::tests::get_dir_entry_for;
    use crate::find::tests::FakeDependencies;

    const POSIX_BASIC_INTERVALS_RE: &str = r".*/ab\{1,3\}c";
    const POSIX_EXTENDED_INTERVALS_RE: &str = r".*/ab{1,3}c";
    const EMACS_AND_POSIX_EXTENDED_KLEENE_PLUS: &str = r".*/ab+c";

    // Variants of fix_up_slashes that properly escape the forward slashes for
    // being in a regex.
    #[cfg(windows)]
    fn fix_up_regex_slashes(re: &str) -> String {
        re.replace("/", r"\\")
    }

    #[cfg(not(windows))]
    fn fix_up_regex_slashes(re: &str) -> String {
        re.to_owned()
    }

    #[test]
    fn case_sensitive_matching() {
        let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
        let matcher =
            RegexMatcher::new(RegexType::Emacs, &fix_up_regex_slashes(".*/ab.BC"), false).unwrap();
        let deps = FakeDependencies::new();
        assert!(!matcher.matches(&abbbc, &mut deps.new_matcher_io()));
    }

    #[test]
    fn case_insensitive_matching() {
        let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
        let matcher =
            RegexMatcher::new(RegexType::Emacs, &fix_up_regex_slashes(".*/ab.BC"), true).unwrap();
        let deps = FakeDependencies::new();
        assert!(matcher.matches(&abbbc, &mut deps.new_matcher_io()));
    }

    #[test]
    fn emacs_regex() {
        // Emacs syntax is mostly the same as POSIX extended but with escaped
        // brace intervals.
        let abbbc = get_dir_entry_for("test_data/simple", "abbbc");

        let matcher = RegexMatcher::new(
            RegexType::Emacs,
            &fix_up_regex_slashes(EMACS_AND_POSIX_EXTENDED_KLEENE_PLUS),
            true,
        )
        .unwrap();
        let deps = FakeDependencies::new();
        assert!(matcher.matches(&abbbc, &mut deps.new_matcher_io()));

        let matcher = RegexMatcher::new(
            RegexType::Emacs,
            &fix_up_regex_slashes(POSIX_EXTENDED_INTERVALS_RE),
            true,
        )
        .unwrap();
        let deps = FakeDependencies::new();
        assert!(!matcher.matches(&abbbc, &mut deps.new_matcher_io()));
    }

    #[test]
    fn posix_basic_regex() {
        let abbbc = get_dir_entry_for("test_data/simple", "abbbc");

        let matcher = RegexMatcher::new(
            RegexType::PosixBasic,
            &fix_up_regex_slashes(POSIX_BASIC_INTERVALS_RE),
            true,
        )
        .unwrap();
        let deps = FakeDependencies::new();
        assert!(matcher.matches(&abbbc, &mut deps.new_matcher_io()));

        let matcher = RegexMatcher::new(
            RegexType::PosixBasic,
            &fix_up_regex_slashes(POSIX_EXTENDED_INTERVALS_RE),
            true,
        )
        .unwrap();
        let deps = FakeDependencies::new();
        assert!(!matcher.matches(&abbbc, &mut deps.new_matcher_io()));
    }

    #[test]
    fn posix_extended_regex() {
        let abbbc = get_dir_entry_for("test_data/simple", "abbbc");

        let matcher = RegexMatcher::new(
            RegexType::PosixExtended,
            &fix_up_regex_slashes(POSIX_EXTENDED_INTERVALS_RE),
            true,
        )
        .unwrap();
        let deps = FakeDependencies::new();
        assert!(matcher.matches(&abbbc, &mut deps.new_matcher_io()));

        let matcher = RegexMatcher::new(
            RegexType::PosixExtended,
            &fix_up_regex_slashes(POSIX_BASIC_INTERVALS_RE),
            true,
        )
        .unwrap();
        let deps = FakeDependencies::new();
        assert!(!matcher.matches(&abbbc, &mut deps.new_matcher_io()));
    }

    #[test]
    fn test_regex_matching_error_sets_exit_code() {
        let entry = get_dir_entry_for("test_data/simple", "abbbc");
        let mut builder = fancy_regex::RegexBuilder::new(r"^.*(\w+)\1.*$");
        builder.backtrack_limit(1);
        let regex = builder.build().unwrap();
        let matcher = RegexMatcher { regex };
        let deps = FakeDependencies::new();
        let mut matcher_io = deps.new_matcher_io();
        assert!(!matcher.matches(&entry, &mut matcher_io));
        assert_eq!(matcher_io.exit_code(), 1);
    }
}
