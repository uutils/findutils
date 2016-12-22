use std::fs::DirEntry;

/// This matcher just prints the name of the file to stdout.
pub struct Printer {}

impl super::Matcher for Printer {
    fn matches(&self, file_info: &DirEntry) -> bool {
        if let Some(x) = file_info.path().to_str() {
            println!("{}", x);
        }
        true
    }

    fn has_side_effects(&self) -> bool {
        true
    }
}
