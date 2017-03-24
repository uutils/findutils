// Copyright 2017 Google Inc.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use walkdir::DirEntry;

use find::matchers::{Matcher, MatcherIO};

/// This matcher just prints the name of the file to stdout.
pub struct Printer;

impl Printer {
    pub fn new() -> Printer {
        Printer {}
    }

    pub fn new_box() -> Box<Matcher> {
        Box::new(Printer::new())
    }
}

impl Matcher for Printer {
    fn matches(&self, file_info: &DirEntry, matcher_io: &mut MatcherIO) -> bool {
        writeln!(matcher_io.deps.get_output().borrow_mut(),
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
    use find::matchers::tests::get_dir_entry_for;
    use find::matchers::Matcher;
    use find::tests::FakeDependencies;
    use find::tests::fix_up_slashes;
    use super::*;

    #[test]
    fn prints() {
        let abbbc = get_dir_entry_for("./test_data/simple", "abbbc");

        let matcher = Printer::new();
        let deps = FakeDependencies::new();
        assert!(matcher.matches(&abbbc, &mut deps.new_matcher_io()));
        assert_eq!(fix_up_slashes("./test_data/simple/abbbc\n"),
                   deps.get_output_as_string());
    }
}
