// Copyright 2017 Google Inc.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use walkdir::DirEntry;

use find::matchers::{Matcher, MatcherIO};

/// This matcher checks the type of the file.
pub struct PruneMatcher;

impl PruneMatcher {
    pub fn new() -> PruneMatcher {
        PruneMatcher {}
    }

    pub fn new_box() -> Box<Matcher> {
        Box::new(PruneMatcher::new())
    }
}

impl Matcher for PruneMatcher {
    fn matches(&self, _: &DirEntry, matcher_io: &mut MatcherIO) -> bool {
        matcher_io.mark_current_dir_to_be_skipped();
        true
    }
}
#[cfg(test)]

mod tests {
    use find::matchers::Matcher;
    use find::matchers::tests::get_dir_entry_for;
    use find::tests::FakeDependencies;
    use super::*;

    #[test]
    fn file_type_matcher() {
        let dir = get_dir_entry_for("test_data", "simple");
        let deps = FakeDependencies::new();

        let mut matcher_io = deps.new_matcher_io();
        assert!(!matcher_io.should_skip_current_dir());
        let matcher = PruneMatcher::new();
        assert!(matcher.matches(&dir, &mut matcher_io));
        assert!(matcher_io.should_skip_current_dir());
    }

}
