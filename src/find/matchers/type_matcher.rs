use std::fs::DirEntry;
use std::fs::FileType;
use std::error::Error;
use std::io::stderr;
use std::io::Write;

/// This matcher checks the type of the file.
pub struct TypeMatcher {
    file_type_fn: fn(&FileType) -> bool,
}

impl TypeMatcher {
    pub fn new(type_string: &str) -> Result<TypeMatcher, Box<Error>> {
        let function = match type_string {
            "f" => FileType::is_file,
            "d" => FileType::is_dir,
            "b" | "c" | "p" | "l" | "s" | "D" => {
                return Err(From::from(format!("Type argument {} not supported yet", type_string)))
            }
            _ => return Err(From::from(format!("Unrecognised type argument {}", type_string))),
        };
        Ok(TypeMatcher { file_type_fn: function })
    }
}

impl super::Matcher for TypeMatcher {
    fn matches(&self, file_info: &DirEntry) -> bool {
        match file_info.file_type() {
            Ok(file_type) => (self.file_type_fn)(&file_type),
            Err(e) => {
                writeln!(&mut stderr(),
                         "Failed to read {}: {}",
                         file_info.path().to_string_lossy(),
                         e)
                    .unwrap();
                false
            }
        }
    }

    fn has_side_effects(&self) -> bool {
        false
    }
}
#[cfg(test)]

mod tests {
    use super::super::tests::get_dir_entry_for;
    use super::TypeMatcher;
    use super::super::Matcher;


    #[test]
    fn file_type_matcher() {
        let file = get_dir_entry_for("test_data/simple", "abbbc");
        let dir = get_dir_entry_for("test_data", "simple");

        let matcher = TypeMatcher::new(&"f".to_string()).unwrap();
        assert!(!matcher.matches(&dir));
        assert!(matcher.matches(&file));
    }

    #[test]
    fn dir_type_matcher() {
        let file = get_dir_entry_for("test_data/simple", "abbbc");
        let dir = get_dir_entry_for("test_data", "simple");

        let matcher = TypeMatcher::new(&"d".to_string()).unwrap();
        assert!(matcher.matches(&dir));
        assert!(!matcher.matches(&file));
    }

    #[test]
    fn cant_create_with_invalid_pattern() {
        let result = TypeMatcher::new(&"xxx".to_string());
        assert!(result.is_err());
    }

}
