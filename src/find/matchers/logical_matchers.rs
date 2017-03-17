// Copyright 2017 Google Inc.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! This modules contains the matchers used for combining other matchers and
//! performing boolean logic on them (and a couple of trivial always-true and
//! always-false matchers). The design is strongly tied to the precedence rules
//! when parsing command-line options (e.g. "-foo -o -bar -baz" is equivalent
//! to "-foo -o ( -bar -baz )", not "( -foo -o -bar ) -baz").
use std::error::Error;
use std::iter::Iterator;
use std::path::PathBuf;
use walkdir::DirEntry;

use find::matchers::{Matcher, MatcherIO};

/// This matcher contains a collection of other matchers. A file only matches
/// if it matches ALL the contained sub-matchers. For sub-matchers that have
/// side effects, the side effects occur in the same order as the sub-matchers
/// were pushed into the collection.
pub struct AndMatcher {
    submatchers: Vec<Box<Matcher>>,
}

impl AndMatcher {
    pub fn new(submatchers: Vec<Box<Matcher>>) -> AndMatcher {
        AndMatcher { submatchers: submatchers }
    }
}


impl Matcher for AndMatcher {
    /// Returns true if all sub-matchers return true. Short-circuiting does take
    /// place. If the nth sub-matcher returns false, then we immediately return
    /// and don't make any further calls.
    fn matches(&self, dir_entry: &DirEntry, matcher_io: &mut MatcherIO) -> bool {
        self.submatchers.iter().all(|x| x.matches(dir_entry, matcher_io))
    }

    fn has_side_effects(&self) -> bool {
        self.submatchers.iter().any(|x| x.has_side_effects())
    }

    fn finished_dir(&self, dir: &PathBuf) {
        for m in &self.submatchers {
            m.finished_dir(dir);
        }
    }

    fn finished(&self) {
        for m in &self.submatchers {
            m.finished();
        }
    }
}

pub struct AndMatcherBuilder {
    submatchers: Vec<Box<Matcher>>,
}

impl AndMatcherBuilder {
    pub fn new() -> AndMatcherBuilder {
        AndMatcherBuilder { submatchers: Vec::new() }
    }

    pub fn new_and_condition(&mut self, matcher: Box<Matcher>) {
        self.submatchers.push(matcher);
    }

    /// Builds a Matcher: consuming the builder in the process.
    pub fn build(mut self) -> Box<Matcher> {
        // special case. If there's only one submatcher, just return that directly
        if self.submatchers.len() == 1 {
            // safe to unwrap: we've just checked the size
            return self.submatchers.pop().unwrap();
        }
        let matcher = Box::new(AndMatcher::new(self.submatchers));
        self.submatchers = Vec::new();
        matcher
    }
}



/// This matcher contains a collection of other matchers. A file matches
/// if it matches any of the contained sub-matchers. For sub-matchers that have
/// side effects, the side effects occur in the same order as the sub-matchers
/// were pushed into the collection.
pub struct OrMatcher {
    submatchers: Vec<Box<Matcher>>,
}

impl OrMatcher {
    pub fn new(submatchers: Vec<Box<Matcher>>) -> OrMatcher {
        OrMatcher { submatchers: submatchers }
    }
}


impl Matcher for OrMatcher {
    /// Returns true if any sub-matcher returns true. Short-circuiting does take
    /// place. If the nth sub-matcher returns true, then we immediately return
    /// and don't make any further calls.
    fn matches(&self, dir_entry: &DirEntry, matcher_io: &mut MatcherIO) -> bool {
        self.submatchers.iter().any(|x| x.matches(dir_entry, matcher_io))
    }

    fn has_side_effects(&self) -> bool {
        self.submatchers.iter().any(|x| x.has_side_effects())
    }

    fn finished_dir(&self, dir: &PathBuf) {
        for m in &self.submatchers {
            m.finished_dir(dir);
        }
    }

    fn finished(&self) {
        for m in &self.submatchers {
            m.finished();
        }
    }
}

pub struct OrMatcherBuilder {
    submatchers: Vec<AndMatcherBuilder>,
}

impl OrMatcherBuilder {
    pub fn new_and_condition(&mut self, matcher: Box<Matcher>) {
        // safe to unwrap. submatchers always has at least one member
        self.submatchers.last_mut().unwrap().new_and_condition(matcher);
    }

    pub fn new_or_condition(&mut self, arg: &str) -> Result<(), Box<Error>> {
        if self.submatchers.last().unwrap().submatchers.is_empty() {
            return Err(From::from(format!("invalid expression; you have used a binary operator \
                                           '{}' with nothing before it.",
                                          arg)));
        }
        self.submatchers.push(AndMatcherBuilder::new());
        Ok(())
    }

    pub fn new() -> OrMatcherBuilder {
        let mut o = OrMatcherBuilder { submatchers: Vec::new() };
        o.submatchers.push(AndMatcherBuilder::new());
        o
    }

    /// Builds a Matcher: consuming the builder in the process.
    pub fn build(mut self) -> Box<Matcher> {
        // Special case: if there's only one submatcher, just return that directly
        if self.submatchers.len() == 1 {
            // safe to unwrap: we've just checked the size
            return self.submatchers.pop().unwrap().build();
        }
        let mut submatchers = vec![];
        for x in self.submatchers {
            submatchers.push(x.build());
        }
        Box::new(OrMatcher::new(submatchers))
    }
}


/// This matcher contains a collection of other matchers. In contrast to
/// `OrMatcher` and `AndMatcher`, all the submatcher objects are called
/// regardless of the results of previous submatchers. This is primarily used
/// for submatchers with side-effects. For such sub-matchers the side effects
/// occur in the same order as the sub-matchers were pushed into the collection.
pub struct ListMatcher {
    submatchers: Vec<Box<Matcher>>,
}

impl ListMatcher {
    pub fn new(submatchers: Vec<Box<Matcher>>) -> ListMatcher {
        ListMatcher { submatchers: submatchers }
    }
}


impl Matcher for ListMatcher {
    /// Calls matches on all submatcher objects, with no short-circuiting.
    /// Returns the result of the call to the final submatcher
    fn matches(&self, dir_entry: &DirEntry, matcher_io: &mut MatcherIO) -> bool {
        let mut rc = false;
        for matcher in &self.submatchers {
            rc = matcher.matches(dir_entry, matcher_io);
        }
        rc
    }

    fn has_side_effects(&self) -> bool {
        self.submatchers.iter().any(|x| x.has_side_effects())
    }

    fn finished_dir(&self, dir: &PathBuf) {
        for m in &self.submatchers {
            m.finished_dir(dir);
        }
    }

    fn finished(&self) {
        for m in &self.submatchers {
            m.finished();
        }
    }
}

pub struct ListMatcherBuilder {
    submatchers: Vec<OrMatcherBuilder>,
}

impl ListMatcherBuilder {
    pub fn new_and_condition(&mut self, matcher: Box<Matcher>) {
        // safe to unwrap. submatchers always has at least one member
        self.submatchers.last_mut().unwrap().new_and_condition(matcher);
    }

    pub fn new_or_condition(&mut self, arg: &str) -> Result<(), Box<Error>> {
        self.submatchers.last_mut().unwrap().new_or_condition(arg)
    }

    pub fn check_new_and_condition(&mut self) -> Result<(), Box<Error>> {
        {
            let child_or_matcher = &self.submatchers.last().unwrap();
            let grandchild_and_matcher = &child_or_matcher.submatchers.last().unwrap();

            if grandchild_and_matcher.submatchers.is_empty() {
                return Err(From::from("invalid expression; you have used a binary operator '-a' \
                                       with nothing before it."));
            }
        }
        Ok(())
    }

    pub fn new_list_condition(&mut self) -> Result<(), Box<Error>> {
        {
            let child_or_matcher = &self.submatchers.last().unwrap();
            let grandchild_and_matcher = &child_or_matcher.submatchers.last().unwrap();

            if grandchild_and_matcher.submatchers.is_empty() {
                return Err(From::from("invalid expression; you have used a binary operator ',' \
                                       with nothing before it."));
            }
        }
        self.submatchers.push(OrMatcherBuilder::new());
        Ok(())
    }

    pub fn new() -> ListMatcherBuilder {
        let mut o = ListMatcherBuilder { submatchers: Vec::new() };
        o.submatchers.push(OrMatcherBuilder::new());
        o
    }

    /// Builds a Matcher: consuming the builder in the process.
    pub fn build(mut self) -> Box<Matcher> {
        // Special case: if there's only one submatcher, just return that directly
        if self.submatchers.len() == 1 {
            // safe to unwrap: we've just checked the size
            return self.submatchers.pop().unwrap().build();
        }
        let mut submatchers = vec![];
        for x in self.submatchers {
            submatchers.push(x.build());
        }
        Box::new(ListMatcher::new(submatchers))
    }
}


/// A simple matcher that always matches.
pub struct TrueMatcher;

impl TrueMatcher {
    pub fn new_box() -> Box<Matcher> {
        Box::new(TrueMatcher {})
    }
}

impl Matcher for TrueMatcher {
    fn matches(&self, _dir_entry: &DirEntry, _: &mut MatcherIO) -> bool {
        true
    }
}

/// A simple matcher that never matches.
pub struct FalseMatcher;

impl Matcher for FalseMatcher {
    fn matches(&self, _dir_entry: &DirEntry, _: &mut MatcherIO) -> bool {
        false
    }
}

impl FalseMatcher {
    pub fn new_box() -> Box<Matcher> {
        Box::new(FalseMatcher {})
    }
}


/// Matcher that wraps another matcher and inverts matching criteria.
pub struct NotMatcher {
    submatcher: Box<Matcher>,
}

impl NotMatcher {
    pub fn new(submatcher: Box<Matcher>) -> NotMatcher {
        NotMatcher { submatcher: submatcher }
    }

    pub fn new_box(submatcher: Box<Matcher>) -> Box<NotMatcher> {
        Box::new(NotMatcher::new(submatcher))
    }
}

impl Matcher for NotMatcher {
    fn matches(&self, dir_entry: &DirEntry, matcher_io: &mut MatcherIO) -> bool {
        !self.submatcher.matches(dir_entry, matcher_io)
    }

    fn has_side_effects(&self) -> bool {
        self.submatcher.has_side_effects()
    }

    fn finished_dir(&self, dir: &PathBuf) {
        self.submatcher.finished_dir(dir);
    }

    fn finished(&self) {
        self.submatcher.finished();
    }
}

#[cfg(test)]

mod tests {
    use walkdir::DirEntry;
    use find::matchers::tests::get_dir_entry_for;
    use super::*;
    use find::matchers::{Matcher, MatcherIO};
    use find::tests::FakeDependencies;

    /// Simple Matcher impl that has side effects
    pub struct HasSideEffects {}

    impl Matcher for HasSideEffects {
        fn matches(&self, _: &DirEntry, _: &mut MatcherIO) -> bool {
            false
        }

        fn has_side_effects(&self) -> bool {
            true
        }
    }

    impl HasSideEffects {
        pub fn new_box() -> Box<Matcher> {
            Box::new(HasSideEffects {})
        }
    }



    #[test]
    fn and_matches_works() {
        let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
        let mut builder = AndMatcherBuilder::new();
        let deps = FakeDependencies::new();

        // start with one matcher returning true
        builder.new_and_condition(TrueMatcher::new_box());
        assert!(builder.build().matches(&abbbc, &mut deps.new_matcher_io()));

        builder = AndMatcherBuilder::new();
        builder.new_and_condition(TrueMatcher::new_box());
        builder.new_and_condition(FalseMatcher::new_box());
        assert!(!builder.build().matches(&abbbc, &mut deps.new_matcher_io()));
    }

    #[test]
    fn or_matches_works() {
        let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
        let mut builder = OrMatcherBuilder::new();
        let deps = FakeDependencies::new();

        // start with one matcher returning false
        builder.new_and_condition(FalseMatcher::new_box());
        assert!(!builder.build().matches(&abbbc, &mut deps.new_matcher_io()));

        let mut builder = OrMatcherBuilder::new();
        builder.new_and_condition(FalseMatcher::new_box());
        builder.new_or_condition("-o").unwrap();
        builder.new_and_condition(TrueMatcher::new_box());
        assert!(builder.build().matches(&abbbc, &mut deps.new_matcher_io()));
    }

    #[test]
    fn list_matches_works() {
        let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
        let mut builder = ListMatcherBuilder::new();
        let deps = FakeDependencies::new();

        // result should always match that of the last pushed submatcher
        builder.new_and_condition(FalseMatcher::new_box());
        assert!(!builder.build().matches(&abbbc, &mut deps.new_matcher_io()));

        builder = ListMatcherBuilder::new();
        builder.new_and_condition(FalseMatcher::new_box());
        builder.new_list_condition().unwrap();
        builder.new_and_condition(TrueMatcher::new_box());
        assert!(builder.build().matches(&abbbc, &mut deps.new_matcher_io()));

        builder = ListMatcherBuilder::new();
        builder.new_and_condition(FalseMatcher::new_box());
        builder.new_list_condition().unwrap();
        builder.new_and_condition(TrueMatcher::new_box());
        builder.new_list_condition().unwrap();
        builder.new_and_condition(FalseMatcher::new_box());
        assert!(!builder.build().matches(&abbbc, &mut deps.new_matcher_io()));
    }

    #[test]
    fn true_matches_works() {
        let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
        let matcher = TrueMatcher {};
        let deps = FakeDependencies::new();

        assert!(matcher.matches(&abbbc, &mut deps.new_matcher_io()));
    }

    #[test]
    fn false_matches_works() {
        let abbbc = get_dir_entry_for("test_data/simple", "abbbc");
        let matcher = FalseMatcher {};
        let deps = FakeDependencies::new();

        assert!(!matcher.matches(&abbbc, &mut deps.new_matcher_io()));
    }

    #[test]
    fn and_has_side_effects_works() {
        let mut builder = AndMatcherBuilder::new();

        // start with one matcher with no side effects false
        builder.new_and_condition(TrueMatcher::new_box());
        assert!(!builder.build().has_side_effects());

        builder = AndMatcherBuilder::new();
        builder.new_and_condition(TrueMatcher::new_box());
        builder.new_and_condition(HasSideEffects::new_box());
        assert!(builder.build().has_side_effects());
    }

    #[test]
    fn or_has_side_effects_works() {
        let mut builder = OrMatcherBuilder::new();

        // start with one matcher with no side effects false
        builder.new_and_condition(TrueMatcher::new_box());
        assert!(!builder.build().has_side_effects());

        builder = OrMatcherBuilder::new();
        builder.new_and_condition(TrueMatcher::new_box());
        builder.new_and_condition(HasSideEffects::new_box());
        assert!(builder.build().has_side_effects());
    }

    #[test]
    fn list_has_side_effects_works() {
        let mut builder = ListMatcherBuilder::new();

        // start with one matcher with no side effects false
        builder.new_and_condition(TrueMatcher::new_box());
        assert!(!builder.build().has_side_effects());

        builder = ListMatcherBuilder::new();
        builder.new_and_condition(TrueMatcher::new_box());
        builder.new_and_condition(HasSideEffects::new_box());
        assert!(builder.build().has_side_effects());
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
        let not_true = NotMatcher::new(TrueMatcher::new_box());
        let not_false = NotMatcher::new(FalseMatcher::new_box());
        let deps = FakeDependencies::new();
        assert!(!not_true.matches(&abbbc, &mut deps.new_matcher_io()));
        assert!(not_false.matches(&abbbc, &mut deps.new_matcher_io()));
    }

    #[test]
    fn not_has_side_effects_works() {
        let has_fx = NotMatcher::new(HasSideEffects::new_box());
        let hasnt_fx = NotMatcher::new(FalseMatcher::new_box());
        assert!(has_fx.has_side_effects());
        assert!(!hasnt_fx.has_side_effects());
    }

}
