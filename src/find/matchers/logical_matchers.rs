use std::fs::DirEntry;


/// This matcher contains a collection of other matchers. A file only matches
/// if it matches ALL the contained sub-matchers. For sub-matchers that have
/// side effects, the side effects occur in the same order as the sub-matchers
/// were pushed into the collection.
pub struct AndMatcher {
    submatchers: Vec<Box<super::Matcher>>,
}

impl AndMatcher {
    pub fn push(&mut self, matcher: Box<super::Matcher>) {
        self.submatchers.push(matcher);
    }

    pub fn new() -> AndMatcher {
        AndMatcher { submatchers: Vec::new() }
    }
}


impl super::Matcher for AndMatcher {
    fn matches(&self, dir_entry: &DirEntry) -> bool {
        self.submatchers.iter().all(|ref x| x.matches(dir_entry))
    }

    fn has_side_effects(&self) -> bool {
        self.submatchers.iter().any(|ref x| x.has_side_effects())
    }
}

/// This matcher contains a collection of other matchers. A file only matches
/// if it matches Any the contained sub-matchers. For sub-matchers that have
/// side effects, the side effects occur in the same order as the sub-matchers
/// were pushed into the collection.
struct OrMatcher {
    submatchers: Vec<Box<super::Matcher>>,
}

impl OrMatcher {
    pub fn push(&mut self, matcher: Box<super::Matcher>) {
        self.submatchers.push(matcher);
    }

    pub fn new() -> OrMatcher {
        OrMatcher { submatchers: Vec::new() }
    }
}


impl super::Matcher for OrMatcher {
    fn matches(&self, dir_entry: &DirEntry) -> bool {
        self.submatchers.iter().any(|ref x| x.matches(dir_entry))
    }

    fn has_side_effects(&self) -> bool {
        self.submatchers.iter().any(|ref x| x.has_side_effects())
    }
}

/// A simple matcher that always matches.
pub struct TrueMatcher {
}

impl super::Matcher for TrueMatcher {
    fn matches(&self, _dir_entry: &DirEntry) -> bool {
        true
    }

    fn has_side_effects(&self) -> bool {
        false
    }
}

/// A simple matcher that never matches.
pub struct FalseMatcher {
}

impl super::Matcher for FalseMatcher {
    fn matches(&self, _dir_entry: &DirEntry) -> bool {
        false
    }

    fn has_side_effects(&self) -> bool {
        false
    }
}

#[cfg(test)]

mod tests {
    use super::super::tests::*;
    use super::AndMatcher;
    use super::OrMatcher;
    use super::TrueMatcher;
    use super::FalseMatcher;
    use super::super::Matcher;

    #[test]
    fn and_matches_works() {
        let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
        let mut matcher = AndMatcher::new();
        let everything = Box::new(TrueMatcher {});
        let nothing = Box::new(FalseMatcher {});

        // start with one matcher returning true
        matcher.push(everything);
        assert!(matcher.matches(&abbbc));
        matcher.push(nothing);
        assert!(!matcher.matches(&abbbc));
    }

    #[test]
    fn or_matches_works() {
        let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
        let mut matcher = OrMatcher::new();
        let matches_everything = Box::new(TrueMatcher {});
        let matches_nothing = Box::new(FalseMatcher {});

        // start with one matcher returning false
        matcher.push(matches_nothing);
        assert!(!matcher.matches(&abbbc));
        matcher.push(matches_everything);
        assert!(matcher.matches(&abbbc));
    }

    #[test]
    fn true_matches_works() {
        let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
        let matcher = TrueMatcher {};

        assert!(matcher.matches(&abbbc));
    }

    #[test]
    fn false_matches_works() {
        let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
        let matcher = FalseMatcher {};

        assert!(!matcher.matches(&abbbc));
    }

    #[test]
    fn and_has_side_effects_works() {
        let mut matcher = AndMatcher::new();
        let no_side_effects = Box::new(TrueMatcher {});
        let side_effects = Box::new(HasSideEfects {});

        // start with one matcher returning false
        matcher.push(no_side_effects);
        assert!(!matcher.has_side_effects());
        matcher.push(side_effects);
        assert!(matcher.has_side_effects());
    }

    #[test]
    fn or_has_side_effects_works() {
        let mut matcher = OrMatcher::new();
        let no_side_effects = Box::new(TrueMatcher {});
        let side_effects = Box::new(HasSideEfects {});

        // start with one matcher returning false
        matcher.push(no_side_effects);
        assert!(!matcher.has_side_effects());
        matcher.push(side_effects);
        assert!(matcher.has_side_effects());
    }

    #[test]
    fn true_has_side_effects_works() {
        let matcher = TrueMatcher {};
        assert!(!matcher.has_side_effects());
    }

    #[test]
    fn false_has_side_effects_works() {
        let matcher = FalseMatcher {};
        assert!(!matcher.has_side_effects());
    }
}
