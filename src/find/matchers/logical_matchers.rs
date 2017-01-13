use std::fs::DirEntry;
use std::error::Error;




/// This matcher contains a collection of other matchers. A file only matches
/// if it matches ALL the contained sub-matchers. For sub-matchers that have
/// side effects, the side effects occur in the same order as the sub-matchers
/// were pushed into the collection.
pub struct AndMatcher {
    submatchers: Vec<Box<super::Matcher>>,
}

impl AndMatcher {
    pub fn new() -> AndMatcher {
        AndMatcher { submatchers: Vec::new() }
    }

    pub fn push(&mut self, matcher: Box<super::Matcher>) {
        self.submatchers.push(matcher);
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

/// This matcher contains a collection of other matchers. A file matches
/// if it matches any of the contained sub-matchers. For sub-matchers that have
/// side effects, the side effects occur in the same order as the sub-matchers
/// were pushed into the collection.
pub struct OrMatcher {
    submatchers: Vec<AndMatcher>,
}

impl OrMatcher {
    pub fn push(&mut self, matcher: Box<super::Matcher>) {
        // safe to unwrap. submatchers always has at least one member
        self.submatchers.last_mut().unwrap().push(matcher);
    }

    pub fn new_ored_criterion(&mut self, arg: &str) -> Result<(), Box<Error>> {
        if self.submatchers.last().unwrap().submatchers.is_empty() {
            return Err(From::from(format!("invalid expression; you have used a binary operator \
                                           '{}' with nothing before it.",
                                          arg)));
        }
        self.submatchers.push(AndMatcher::new());
        Ok(())
    }

    pub fn new() -> OrMatcher {
        let mut o = OrMatcher { submatchers: Vec::new() };
        o.submatchers.push(AndMatcher::new());
        o
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

/// Matcher that wraps another matcher and inverts matching criteria.
pub struct NotMatcher {
    submatcher: Box<super::Matcher>,
}

impl NotMatcher {
    pub fn new(submatcher: Box<super::Matcher>) -> NotMatcher {
        NotMatcher { submatcher: submatcher }
    }
}

impl super::Matcher for NotMatcher {
    fn matches(&self, dir_entry: &DirEntry) -> bool {
        !self.submatcher.matches(dir_entry)
    }

    fn has_side_effects(&self) -> bool {
        self.submatcher.has_side_effects()
    }
}

#[cfg(test)]

mod tests {
    use super::super::tests::*;
    use super::*;
    use super::super::Matcher;
    use std::fs::DirEntry;

    /// Simple Matcher impl that has side effects
    pub struct HasSideEffects {}

    impl Matcher for HasSideEffects {
        fn matches(&self, _: &DirEntry) -> bool {
            false
        }

        fn has_side_effects(&self) -> bool {
            true
        }
    }



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
        matcher.new_ored_criterion("-o").unwrap();
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
        let side_effects = Box::new(HasSideEffects {});

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
        let side_effects = Box::new(HasSideEffects {});

        // start with one matcher returning false
        matcher.push(no_side_effects);
        assert!(!matcher.has_side_effects());
        matcher.new_ored_criterion("-o").unwrap();
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

    #[test]
    fn not_matches_works() {
        let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
        let not_true = NotMatcher::new(Box::new(TrueMatcher {}));
        let not_false = NotMatcher::new(Box::new(FalseMatcher {}));
        assert!(!not_true.matches(&abbbc));
        assert!(not_false.matches(&abbbc));
    }

    #[test]
    fn not_has_side_effects_works() {
        let has_fx = NotMatcher::new(Box::new(HasSideEffects {}));
        let hasnt_fx = NotMatcher::new(Box::new(FalseMatcher {}));
        assert!(has_fx.has_side_effects());
        assert!(!hasnt_fx.has_side_effects());
    }

}
