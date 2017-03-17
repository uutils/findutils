// Copyright 2017 Google Inc.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use glob::Pattern;
use glob::PatternError;
use walkdir::DirEntry;

use find::matchers::{Matcher, MatcherIO};

/// This matcher makes a case-sensitive comparison of the name against a
/// shell wildcard pattern. See `glob::Pattern` for details on the exact
/// syntax.
pub struct NameMatcher {
    pattern: Pattern,
}

impl NameMatcher {
    pub fn new(pattern_string: &str) -> Result<NameMatcher, PatternError> {
        let p = Pattern::new(pattern_string)?;
        Ok(NameMatcher { pattern: p })
    }

    pub fn new_box(pattern_string: &str) -> Result<Box<Matcher>, PatternError> {
        Ok(Box::new(NameMatcher::new(pattern_string)?))
    }
}

impl Matcher for NameMatcher {
    fn matches(&self, file_info: &DirEntry, _: &mut MatcherIO) -> bool {
        self.pattern.matches(file_info.file_name().to_string_lossy().as_ref())
    }
}

/// This matcher makes a case-insensitive comparison of the name against a
/// shell wildcard pattern. See `glob::Pattern` for details on the exact
/// syntax.
pub struct CaselessNameMatcher {
    pattern: Pattern,
}

impl CaselessNameMatcher {
    pub fn new(pattern_string: &str) -> Result<CaselessNameMatcher, PatternError> {
        let p = Pattern::new(&pattern_string.to_lowercase())?;
        Ok(CaselessNameMatcher { pattern: p })
    }

    pub fn new_box(pattern_string: &str) -> Result<Box<Matcher>, PatternError> {
        Ok(Box::new(CaselessNameMatcher::new(pattern_string)?))
    }
}

impl super::Matcher for CaselessNameMatcher {
    fn matches(&self, file_info: &DirEntry, _: &mut MatcherIO) -> bool {
        self.pattern
            .matches(file_info.file_name().to_string_lossy().to_lowercase().as_ref())
    }
}


#[cfg(test)]

mod tests {
    use find::matchers::Matcher;
    use find::matchers::tests::get_dir_entry_for;
    use find::tests::FakeDependencies;
    use super::*;


    #[test]
    fn matching_with_wrong_case_returns_false() {
        let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
        let matcher = NameMatcher::new(&"A*C".to_string()).unwrap();
        let deps = FakeDependencies::new();
        assert!(!matcher.matches(&abbbc, &mut deps.new_matcher_io()));
    }

    #[test]
    fn matching_with_right_case_returns_true() {
        let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
        let matcher = NameMatcher::new(&"abb?c".to_string()).unwrap();
        let deps = FakeDependencies::new();
        assert!(matcher.matches(&abbbc, &mut deps.new_matcher_io()));
    }

    #[test]
    fn not_matching_returns_false() {
        let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
        let matcher = NameMatcher::new(&"should't match".to_string()).unwrap();
        let deps = FakeDependencies::new();
        assert!(!matcher.matches(&abbbc, &mut deps.new_matcher_io()));
    }

    #[test]
    fn cant_create_with_invalid_pattern() {
        let result = NameMatcher::new(&"a**c".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn caseless_matching_with_wrong_case_returns_true() {
        let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
        let matcher = CaselessNameMatcher::new(&"A*C".to_string()).unwrap();
        let deps = FakeDependencies::new();
        assert!(matcher.matches(&abbbc, &mut deps.new_matcher_io()));
    }

    #[test]
    fn caseless_matching_with_right_case_returns_true() {
        let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
        let matcher = CaselessNameMatcher::new(&"abb?c".to_string()).unwrap();
        let deps = FakeDependencies::new();
        assert!(matcher.matches(&abbbc, &mut deps.new_matcher_io()));
    }

    #[test]
    fn caseless_not_matching_returns_false() {
        let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
        let matcher = CaselessNameMatcher::new(&"should't match".to_string()).unwrap();
        let deps = FakeDependencies::new();
        assert!(!matcher.matches(&abbbc, &mut deps.new_matcher_io()));
    }

    #[test]
    fn caseless_cant_create_with_invalid_pattern() {
        let result = CaselessNameMatcher::new(&"a**c".to_string());
        assert!(result.is_err());
    }
}
