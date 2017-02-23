use super::PathInfo;

use super::MatcherIO;

/// This matcher checks the type of the file.
pub struct PruneMatcher {
}

impl PruneMatcher {
    pub fn new() -> PruneMatcher {
        PruneMatcher {}
    }

    pub fn new_box() -> Box<super::Matcher> {
        Box::new(PruneMatcher::new())
    }
}

impl super::Matcher for PruneMatcher {
    fn matches(&self, _: &PathInfo, matcher_io: &mut MatcherIO) -> bool {
        matcher_io.mark_current_dir_to_be_skipped();
        return true;
    }

    fn has_side_effects(&self) -> bool {
        false
    }
}
#[cfg(test)]

mod tests {
    use super::super::tests::get_dir_entry_for;
    use super::PruneMatcher;
    use super::super::Matcher;
    use find::test::FakeDependencies;

    #[test]
    fn file_type_matcher() {
        let dir = get_dir_entry_for("test_data", "simple");
        let deps = FakeDependencies::new();

        let mut matcher_io = deps.new_side_effects();
        assert!(!matcher_io.should_skip_current_dir());
        let matcher = PruneMatcher::new();
        assert!(matcher.matches(&dir, &mut matcher_io));
        assert!(matcher_io.should_skip_current_dir());
    }

}
