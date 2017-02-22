use super::PathInfo;

use super::SideEffectRefs;

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
    fn matches(&self, _: &PathInfo, side_effects: &mut SideEffectRefs) -> bool {
        side_effects.should_skip_current_dir = true;
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

        let mut side_effects = deps.new_side_effects();
        assert!(!side_effects.should_skip_current_dir);
        let matcher = PruneMatcher::new();
        assert!(matcher.matches(&dir, &mut side_effects));
        assert!(side_effects.should_skip_current_dir);
    }

}
