use std::error::Error;
use std::io::{stderr, Write};
use walkdir::DirEntry;

use find::matchers::{ComparableValue, Matcher, MatcherIO};

#[derive(Clone, Copy, Debug)]
enum Unit {
    Byte,
    TwoByteWord,
    Block,
    KiloByte,
    MegaByte,
    GigaByte,
}

impl Unit {
    fn from_string(s: &str) -> Result<Unit, Box<Error>> {
        Ok(match s {
            "c" => Unit::Byte,
            "w" => Unit::TwoByteWord,
            "" | "b" => Unit::Block,
            "k" => Unit::KiloByte,
            "M" => Unit::MegaByte,
            "G" => Unit::GigaByte,
            _ => {
                return Err(From::from(format!("Invalid suffix {} for -size. Only allowed \
                                               values are <nothing>, b, c, w, k, M or G",
                                              s)));
            }
        })
    }
}

fn byte_size_to_unit_size(unit: Unit, byte_size: u64) -> u64 {
    // Short circuit (to avoid a overflow error when subtracting 1 later on)
    if byte_size == 0 {
        return 0;
    }
    let divisor = match unit {
        Unit::Byte => 1,
        Unit::TwoByteWord => 2,
        Unit::Block => 512,
        Unit::KiloByte => 1024,
        Unit::MegaByte => 1024 * 1024,
        Unit::GigaByte => 1024 * 1024 * 1024,
    };
    // Skip pointless arithmetic.
    if divisor == 1 {
        return byte_size;
    }
    // We want to round up (e.g. 1 byte - 1024 bytes = 1k.
    // 1025 bytes to 2048 bytes = 2k etc.
    (byte_size - 1) / divisor + 1
}

/// Matcher that checks whether a file's size if {less than | equal to | more than}
/// N units in size.
pub struct SizeMatcher {
    value_to_match: ComparableValue,
    unit: Unit,
}

impl SizeMatcher {
    pub fn new(value_to_match: ComparableValue,
               suffix_string: &str)
               -> Result<SizeMatcher, Box<Error>> {
        Ok(SizeMatcher {
            unit: try!(Unit::from_string(suffix_string)),
            value_to_match: value_to_match,
        })
    }

    pub fn new_box(value_to_match: ComparableValue,
                   suffix_string: &str)
                   -> Result<Box<Matcher>, Box<Error>> {
        Ok(Box::new(try!(SizeMatcher::new(value_to_match, suffix_string))))
    }
}

impl Matcher for SizeMatcher {
    fn matches(&self, file_info: &DirEntry, _: &mut MatcherIO) -> bool {
        match file_info.metadata() {
            Ok(metadata) => {
                self.value_to_match
                    .matches(byte_size_to_unit_size(self.unit, metadata.len()))
            }
            Err(e) => {
                writeln!(&mut stderr(),
                         "Error getting file size for {}: {}",
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
    use find::matchers::{ComparableValue, Matcher};
    use find::matchers::tests::get_dir_entry_for;
    use find::tests::FakeDependencies;
    use super::*;
    // need to explicitly use non-pub members
    use super::{byte_size_to_unit_size, Unit};

    #[test]
    fn test_byte_size_to_unit_size() {
        assert_eq!(byte_size_to_unit_size(Unit::KiloByte, 0), 0);
        assert_eq!(byte_size_to_unit_size(Unit::KiloByte, 1), 1);
        assert_eq!(byte_size_to_unit_size(Unit::KiloByte, 1024), 1);
        assert_eq!(byte_size_to_unit_size(Unit::KiloByte, 1025), 2);
        assert_eq!(byte_size_to_unit_size(Unit::Byte, 1025), 1025);
        assert_eq!(byte_size_to_unit_size(Unit::TwoByteWord, 1025), 513);
        assert_eq!(byte_size_to_unit_size(Unit::Block, 1025), 3);
        assert_eq!(byte_size_to_unit_size(Unit::KiloByte, 1025), 2);
        assert_eq!(byte_size_to_unit_size(Unit::MegaByte, 1024 * 1024 + 1), 2);
        assert_eq!(byte_size_to_unit_size(Unit::GigaByte, 1024 * 1024 * 1024 + 1),
                   2);
    }

    #[test]
    fn unit_from_string() {
        assert_eq!(byte_size_to_unit_size(Unit::from_string("c").unwrap(), 2),
                   2);
        assert_eq!(byte_size_to_unit_size(Unit::from_string("w").unwrap(), 3),
                   2);
        assert_eq!(byte_size_to_unit_size(Unit::from_string("b").unwrap(), 513),
                   2);
        assert_eq!(byte_size_to_unit_size(Unit::from_string("").unwrap(), 513),
                   2);
        assert_eq!(byte_size_to_unit_size(Unit::from_string("k").unwrap(), 1025),
                   2);
        assert_eq!(byte_size_to_unit_size(Unit::from_string("M").unwrap(), 1024 * 1024 + 1),
                   2);
        assert_eq!(byte_size_to_unit_size(Unit::from_string("G").unwrap(), 2024 * 1024 * 1024 + 1),
                   2);
    }

    #[test]
    fn size_matcher_bad_unit() {
        if let Err(e) = SizeMatcher::new(ComparableValue::EqualTo(2), "xyz") {
            assert!(e.description().contains("Invalid suffix") && e.description().contains("xyz"),
                    "bad description: {}",
                    e);
        } else {
            panic!("parsing a unit string should fail");
        }
    }

    #[test]
    fn size_matcher() {
        let file_info = get_dir_entry_for("./test_data/size", "512bytes");

        let equal_to_2_blocks = SizeMatcher::new(ComparableValue::EqualTo(2), "b").unwrap();
        let equal_to_1_blocks = SizeMatcher::new(ComparableValue::EqualTo(1), "b").unwrap();
        let deps = FakeDependencies::new();

        assert!(!equal_to_2_blocks.matches(&file_info, &mut deps.new_matcher_io()),
                "512-byte file should not match size of 2 blocks");
        assert!(equal_to_1_blocks.matches(&file_info, &mut deps.new_matcher_io()),
                "512-byte file should match size of 1 block");
    }




}
