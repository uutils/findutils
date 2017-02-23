use glob::Pattern;
use glob::PatternError;

use find::matchers::{Matcher, PathInfo, MatcherIO};



/// This matcher makes a case-insensitive comparison of the name against a
/// shell wildcard pattern. See glob::Pattern for details on the exact
/// syntax.
pub struct CaselessNameMatcher {
    pattern: Pattern,
}

impl CaselessNameMatcher {
    pub fn new(pattern_string: &str) -> Result<CaselessNameMatcher, PatternError> {
        let p = try!(Pattern::new(&pattern_string.to_lowercase()));
        Ok(CaselessNameMatcher { pattern: p })
    }

    pub fn new_box(pattern_string: &str) -> Result<Box<Matcher>, PatternError> {
        Ok(Box::new(try!(CaselessNameMatcher::new(pattern_string))))
    }
}

impl super::Matcher for CaselessNameMatcher {
    fn matches(&self, file_info: &PathInfo, _: &mut MatcherIO) -> bool {
        if let Ok(x) = file_info.file_name().into_string() {
            return self.pattern.matches(x.to_lowercase().as_ref());
        }
        false
    }

    fn has_side_effects(&self) -> bool {
        false
    }
}


#[cfg(test)]

mod tests {
    use find::matchers::Matcher;
    use find::matchers::tests::get_dir_entry_for;
    use find::tests::FakeDependencies;
    use super::*;

    #[test]
    fn matching_with_wrong_case_returns_true() {
        let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
        let matcher = CaselessNameMatcher::new(&"A*C".to_string()).unwrap();
        let deps = FakeDependencies::new();
        assert!(matcher.matches(&abbbc, &mut deps.new_side_effects()));
    }

    #[test]
    fn matching_with_right_case_returns_true() {
        let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
        let matcher = CaselessNameMatcher::new(&"abb?c".to_string()).unwrap();
        let deps = FakeDependencies::new();
        assert!(matcher.matches(&abbbc, &mut deps.new_side_effects()));
    }

    #[test]
    fn not_matching_returns_false() {
        let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
        let matcher = CaselessNameMatcher::new(&"should't match".to_string()).unwrap();
        let deps = FakeDependencies::new();
        assert!(!matcher.matches(&abbbc, &mut deps.new_side_effects()));
    }

    #[test]
    fn cant_create_with_invalid_pattern() {
        let result = CaselessNameMatcher::new(&"a**c".to_string());
        assert!(result.is_err());
    }

}
