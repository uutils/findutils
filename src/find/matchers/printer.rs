use super::PathInfo;
use super::SideEffectRefs;

/// This matcher just prints the name of the file to stdout.
pub struct Printer {
}

impl Printer {
    pub fn new() -> Printer {
        Printer {}
    }

    pub fn new_box() -> Box<super::Matcher> {
        Box::new(Printer::new())
    }
}

impl super::Matcher for Printer {
    fn matches(&self, file_info: &PathInfo, side_effects: &mut SideEffectRefs) -> bool {
        writeln!(side_effects.deps.get_output().borrow_mut(),
                 "{}",
                 file_info.path().to_string_lossy())
            .unwrap();
        true
    }

    fn has_side_effects(&self) -> bool {
        true
    }
}

#[cfg(test)]

mod tests {
    use super::super::tests::*;
    use super::Printer;
    use super::super::Matcher;
    use find::test::FakeDependencies;

    #[test]
    fn prints() {
        let abbbc = get_dir_entry_for("./test_data/simple", "abbbc");

        let matcher = Printer::new();
        let deps = FakeDependencies::new();
        assert!(matcher.matches(&abbbc, &mut deps.new_side_effects()));
        assert_eq!("./test_data/simple/abbbc\n", deps.get_output_as_string());
    }
}
