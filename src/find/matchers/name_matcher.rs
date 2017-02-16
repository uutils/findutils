use glob::Pattern;
use glob::PatternError;

use super::PathInfo;

/// This matcher makes a case-sensitive comparison of the name against a
/// shell wildcard pattern. See glob::Pattern for details on the exact
/// syntax.
pub struct NameMatcher {
    pattern: Pattern,
}

impl NameMatcher {
    pub fn new(pattern_string: &str) -> Result<NameMatcher, PatternError> {
        let p = try!(Pattern::new(pattern_string));
        Ok(NameMatcher { pattern: p })
    }

    pub fn new_box(pattern_string: &str) -> Result<Box<super::Matcher>, PatternError> {
        Ok(Box::new(try!(NameMatcher::new(pattern_string))))
    }
}

impl super::Matcher for NameMatcher {
    fn matches(&self, file_info: &PathInfo) -> bool {
        if let Ok(x) = file_info.file_name().into_string() {
            return self.pattern.matches(x.as_ref());
        }
        false
    }

    fn has_side_effects(&self) -> bool {
        false
    }
}

#[cfg(test)]

mod tests {
    use super::super::tests::get_dir_entry_for;
    use super::NameMatcher;
    use super::super::Matcher;


    #[test]
    fn matching_with_wrong_case_returns_false() {
        let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
        let matcher = NameMatcher::new(&"A*C".to_string()).unwrap();
        assert!(!matcher.matches(&abbbc));
    }

    #[test]
    fn matching_with_right_case_returns_true() {
        let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
        let matcher = NameMatcher::new(&"abb?c".to_string()).unwrap();
        assert!(matcher.matches(&abbbc));
    }

    #[test]
    fn not_matching_returns_false() {
        let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
        let matcher = NameMatcher::new(&"should't match".to_string()).unwrap();
        assert!(!matcher.matches(&abbbc));
    }

    #[test]
    fn cant_create_with_invalid_pattern() {
        let result = NameMatcher::new(&"a**c".to_string());
        assert!(result.is_err());
    }

}
