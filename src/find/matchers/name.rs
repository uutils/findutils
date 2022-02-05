// Copyright 2017 Google Inc.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::io::stderr;
use std::io::Write;
use std::path::PathBuf;

use glob::Pattern;
use glob::PatternError;
use walkdir::DirEntry;

use super::{Matcher, MatcherIO};

fn read_link_target(file_info: &DirEntry) -> Option<PathBuf> {
    match file_info.path().read_link() {
        Ok(target) => Some(target),
        Err(err) => {
            // If it's not a symlink, then it's not an error that should be
            // shown.
            if err.kind() != std::io::ErrorKind::InvalidInput {
                writeln!(
                    &mut stderr(),
                    "Error reading target of {}: {}",
                    file_info.path().display(),
                    err
                )
                .unwrap();
            }

            None
        }
    }
}

/// This matcher makes a case-sensitive comparison of the name against a
/// shell wildcard pattern. See `glob::Pattern` for details on the exact
/// syntax.
pub struct NameMatcher {
    pattern: Pattern,
    match_link_target: bool,
}

impl NameMatcher {
    pub fn new(pattern_string: &str, match_link_target: bool) -> Result<NameMatcher, PatternError> {
        let p = Pattern::new(pattern_string)?;
        Ok(NameMatcher {
            pattern: p,
            match_link_target,
        })
    }

    pub fn new_box(
        pattern_string: &str,
        match_link_target: bool,
    ) -> Result<Box<dyn Matcher>, PatternError> {
        Ok(Box::new(NameMatcher::new(
            pattern_string,
            match_link_target,
        )?))
    }
}

impl Matcher for NameMatcher {
    fn matches(&self, file_info: &DirEntry, _: &mut MatcherIO) -> bool {
        if self.match_link_target {
            if let Some(target) = read_link_target(file_info) {
                self.pattern.matches(&target.to_string_lossy())
            } else {
                false
            }
        } else {
            self.pattern
                .matches(file_info.file_name().to_string_lossy().as_ref())
        }
    }
}

/// This matcher makes a case-insensitive comparison of the name against a
/// shell wildcard pattern. See `glob::Pattern` for details on the exact
/// syntax.
pub struct CaselessNameMatcher {
    pattern: Pattern,
    match_link_target: bool,
}

impl CaselessNameMatcher {
    pub fn new(
        pattern_string: &str,
        match_link_target: bool,
    ) -> Result<CaselessNameMatcher, PatternError> {
        let p = Pattern::new(&pattern_string.to_lowercase())?;
        Ok(CaselessNameMatcher {
            pattern: p,
            match_link_target,
        })
    }

    pub fn new_box(
        pattern_string: &str,
        match_link_target: bool,
    ) -> Result<Box<dyn Matcher>, PatternError> {
        Ok(Box::new(CaselessNameMatcher::new(
            pattern_string,
            match_link_target,
        )?))
    }
}

impl super::Matcher for CaselessNameMatcher {
    fn matches(&self, file_info: &DirEntry, _: &mut MatcherIO) -> bool {
        if self.match_link_target {
            if let Some(target) = read_link_target(file_info) {
                self.pattern
                    .matches(&target.to_string_lossy().to_lowercase())
            } else {
                false
            }
        } else {
            self.pattern
                .matches(&file_info.file_name().to_string_lossy().to_lowercase())
        }
    }
}

#[cfg(test)]

mod tests {
    use super::*;
    use crate::find::matchers::tests::get_dir_entry_for;
    use crate::find::matchers::Matcher;
    use crate::find::tests::FakeDependencies;

    use std::io::ErrorKind;

    #[cfg(unix)]
    use std::os::unix::fs::symlink;

    #[cfg(windows)]
    use std::os::windows::fs::{symlink_dir, symlink_file};

    fn create_file_link() {
        #[cfg(unix)]
        if let Err(e) = symlink("abbbc", "test_data/links/link-f") {
            if e.kind() != ErrorKind::AlreadyExists {
                panic!("Failed to create sym link: {:?}", e);
            }
        }
        #[cfg(windows)]
        if let Err(e) = symlink_file("abbbc", "test_data/links/link-f") {
            if e.kind() != ErrorKind::AlreadyExists {
                panic!("Failed to create sym link: {:?}", e);
            }
        }
    }

    #[test]
    fn matching_with_wrong_case_returns_false() {
        let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
        let matcher = NameMatcher::new(&"A*C".to_string(), false).unwrap();
        let deps = FakeDependencies::new();
        assert!(!matcher.matches(&abbbc, &mut deps.new_matcher_io()));
    }

    #[test]
    fn matching_with_right_case_returns_true() {
        let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
        let matcher = NameMatcher::new(&"abb?c".to_string(), false).unwrap();
        let deps = FakeDependencies::new();
        assert!(matcher.matches(&abbbc, &mut deps.new_matcher_io()));
    }

    #[test]
    fn not_matching_returns_false() {
        let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
        let matcher = NameMatcher::new(&"shouldn't match".to_string(), false).unwrap();
        let deps = FakeDependencies::new();
        assert!(!matcher.matches(&abbbc, &mut deps.new_matcher_io()));
    }

    #[test]
    fn matches_against_link_file_name() {
        create_file_link();

        let link_f = get_dir_entry_for("test_data/links", "link-f");
        let matcher = NameMatcher::new("link?f", false).unwrap();
        let deps = FakeDependencies::new();
        assert!(matcher.matches(&link_f, &mut deps.new_matcher_io()));
    }

    #[test]
    fn matches_against_link_target_if_requested() {
        create_file_link();

        let link_f = get_dir_entry_for("test_data/links", "link-f");
        let matcher = NameMatcher::new("ab?bc", true).unwrap();
        let deps = FakeDependencies::new();
        assert!(matcher.matches(&link_f, &mut deps.new_matcher_io()));
    }

    #[test]
    fn cant_create_with_invalid_pattern() {
        let result = NameMatcher::new(&"a**c".to_string(), false);
        assert!(result.is_err());
    }

    #[test]
    fn caseless_matching_with_wrong_case_returns_true() {
        let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
        let matcher = CaselessNameMatcher::new(&"A*C".to_string(), false).unwrap();
        let deps = FakeDependencies::new();
        assert!(matcher.matches(&abbbc, &mut deps.new_matcher_io()));
    }

    #[test]
    fn caseless_matching_with_right_case_returns_true() {
        let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
        let matcher = CaselessNameMatcher::new(&"abb?c".to_string(), false).unwrap();
        let deps = FakeDependencies::new();
        assert!(matcher.matches(&abbbc, &mut deps.new_matcher_io()));
    }

    #[test]
    fn caseless_not_matching_returns_false() {
        let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
        let matcher = CaselessNameMatcher::new(&"shouldn't match".to_string(), false).unwrap();
        let deps = FakeDependencies::new();
        assert!(!matcher.matches(&abbbc, &mut deps.new_matcher_io()));
    }

    #[test]
    fn caseless_matches_against_link_file_name() {
        create_file_link();

        let link_f = get_dir_entry_for("test_data/links", "link-f");
        let matcher = CaselessNameMatcher::new("linK?f", false).unwrap();
        let deps = FakeDependencies::new();
        assert!(matcher.matches(&link_f, &mut deps.new_matcher_io()));
    }

    #[test]
    fn caseless_matches_against_link_target_if_requested() {
        create_file_link();

        let link_f = get_dir_entry_for("test_data/links", "link-f");
        let matcher = CaselessNameMatcher::new("AbB?c", true).unwrap();
        let deps = FakeDependencies::new();
        assert!(matcher.matches(&link_f, &mut deps.new_matcher_io()));
    }

    #[test]
    fn caseless_cant_create_with_invalid_pattern() {
        let result = CaselessNameMatcher::new(&"a**c".to_string(), false);
        assert!(result.is_err());
    }
}
