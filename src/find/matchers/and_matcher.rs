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



#[cfg(test)]

mod tests {
    use super::super::tests::*;
    use super::AndMatcher;
    use super::super::Matcher;
    use super::super::printer::Printer;


    #[test]
    fn matches_works() {
        let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
        let mut matcher = AndMatcher::new();
        let everything = Box::new(MatchEverything {});
        let nothing = Box::new(MatchNothing {});

        // start with one matcher returning true
        matcher.push(everything);
        assert!(matcher.matches(&abbbc));
        matcher.push(nothing);
        assert!(!matcher.matches(&abbbc));
    }

    #[test]
    fn has_side_effects_works() {
        let mut matcher = AndMatcher::new();
        let no_side_effects = Box::new(MatchEverything {});
        let side_effects = Box::new(Printer {});

        // start with one matcher returning false
        matcher.push(no_side_effects);
        assert!(!matcher.has_side_effects());
        matcher.push(side_effects);
        assert!(matcher.has_side_effects());
    }
}
