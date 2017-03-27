// Copyright 2017 Google Inc.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! find's permission matching uses a very unix-centric approach, that would
//! be tricky to both implement and use on a windows platform. So we don't
//! even try.

use std::error::Error;
use std::io::{stderr, Write};
#[cfg(unix)]
use std::str::FromStr;
use walkdir::DirEntry;

use find::matchers::{Matcher, MatcherIO};


#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg(unix)]
pub enum ComparisonType {
    /// mode bits have to match exactly
    Exact,
    /// all specified mode bits must be set. Others can be as well
    AtLeast,
    /// at least one of the specified bits must be set (or if no bits are
    /// specified then any mode will match)
    AnyOf,
}

#[cfg(unix)]
impl FromStr for ComparisonType {
    type Err = Box<Error>;
    fn from_str(s: &str) -> Result<ComparisonType, Box<Error>> {
        Ok(match s {
            "" => ComparisonType::Exact,
            "-" => ComparisonType::AtLeast,
            "/" => ComparisonType::AnyOf,
            _ => {
                return Err(From::from(format!("Invalid prefix {} for -perm. Only allowed \
                                               values are <nothing>, /, or -",
                                              s)));
            }
        })
    }
}

#[cfg(unix)]
impl ComparisonType {
    fn mode_bits_match(&self, pattern: u32, value: u32) -> bool {
        match *self {
            ComparisonType::Exact => (0o7777 & value) == pattern,
            ComparisonType::AtLeast => (value & pattern) == pattern,
            ComparisonType::AnyOf => pattern == 0 || (value & pattern) > 0,
        }
    }
}

#[cfg(unix)]
mod parsing {
    use regex::Regex;
    use std::error::Error;
    use super::*;

    // We need to be able to parse strings like /u+rw,g+w,o=w. Specifically
    // we have a prefix, as per ComparisonType. then combinations of (u, g or o
    // followed by a + or =, followed by combinations of r w and x) separated by
    // commas. Writing a hard-coded parser is easier to understand - and probably
    // shorter :-) -  than the abomination required to do this as a regex
    enum ParserState {
        Beginning, // we're at the start, now waiting for /, -, a, u, g or o
        GatheringCategories, // expecting u, g or o to set categories, or =/+ to switch to...
        GatheringPermissions, // expecting r, w, or x
    }

    struct Parser<'a> {
        state: ParserState,
        bit_pattern: u32,
        comparison_type: ComparisonType,
        string_pattern: &'a str,
        category_bit_pattern: u32,
    }


    impl<'a> Parser<'a> {
        fn new(string_pattern: &'a str) -> Parser<'a> {
            Parser {
                state: ParserState::Beginning,
                bit_pattern: 0,
                comparison_type: ComparisonType::Exact,
                string_pattern: string_pattern,
                category_bit_pattern: 0,
            }
        }

        fn error(&self) -> Result<(), Box<Error>> {
            Err(From::from(format!("invalid mode '{}'", self.string_pattern)))
        }

        fn handle_char(&mut self, char: &char) -> Result<(), Box<Error>> {
            if let ParserState::Beginning = self.state {};


            match *char {
                '-' => {
                    if let ParserState::Beginning = self.state {
                        self.comparison_type = ComparisonType::AtLeast;
                        self.state = ParserState::GatheringCategories;
                    } else {
                        return self.error();
                    }
                }
                '/' => {
                    if let ParserState::Beginning = self.state {
                        self.comparison_type = ComparisonType::AnyOf;
                        self.state = ParserState::GatheringCategories;
                    } else {
                        return self.error();
                    }
                }
                'a' => {
                    match self.state {
                        ParserState::Beginning |
                        ParserState::GatheringCategories => {
                            self.state = ParserState::GatheringCategories;
                            self.category_bit_pattern = 0o111;
                        }
                        _ => {
                            return self.error();
                        }
                    };
                }
                'g' => {
                    match self.state {
                        ParserState::Beginning |
                        ParserState::GatheringCategories => {
                            self.state = ParserState::GatheringCategories;
                            self.category_bit_pattern |= 0o010;
                        }
                        _ => {
                            return self.error();
                        }
                    };
                }
                'u' => {
                    match self.state {
                        ParserState::Beginning |
                        ParserState::GatheringCategories => {
                            self.state = ParserState::GatheringCategories;
                            self.category_bit_pattern |= 0o100;
                        }
                        _ => {
                            return self.error();
                        }
                    };
                }
                'o' => {
                    match self.state {
                        ParserState::Beginning |
                        ParserState::GatheringCategories => {
                            self.state = ParserState::GatheringCategories;
                            self.category_bit_pattern |= 0o001;
                        }
                        _ => {
                            return self.error();
                        }
                    };
                }
                '=' | '+' => {
                    if let ParserState::GatheringCategories = self.state {
                        self.state = ParserState::GatheringPermissions;
                    } else {
                        return self.error();
                    }
                }
                'r' => {
                    if let ParserState::GatheringPermissions = self.state {
                        self.bit_pattern |= self.category_bit_pattern << 2;
                    } else {
                        return self.error();
                    }
                }
                'w' => {
                    if let ParserState::GatheringPermissions = self.state {
                        self.bit_pattern |= self.category_bit_pattern << 1;
                    } else {
                        return self.error();
                    }
                }
                'x' => {
                    if let ParserState::GatheringPermissions = self.state {
                        self.bit_pattern |= self.category_bit_pattern;
                    } else {
                        return self.error();
                    }
                }
                't' => {
                    if let ParserState::GatheringPermissions = self.state {
                        self.bit_pattern |= 0o1000;
                    } else {
                        return self.error();
                    }
                }
                's' => {
                    if let ParserState::GatheringPermissions = self.state {
                        // if we're setting group bits, then set the set-group-id bit
                        if (self.category_bit_pattern & 0o010) == 0o010 {
                            self.bit_pattern |= 0o2000;
                        }
                        // if we're setting user bits, then set the set-user-id bit
                        if (self.category_bit_pattern & 0o100) == 0o100 {
                            self.bit_pattern |= 0o4000;
                        }
                    } else {
                        return self.error();
                    }
                }
                ',' => {
                    if let ParserState::GatheringPermissions = self.state {
                        self.state = ParserState::GatheringCategories;
                        self.category_bit_pattern = 0;
                    } else {
                        return self.error();
                    }
                }
                _ => {
                    return self.error();
                }
            };
            Ok(())
        }
    }

    pub fn parse(string_value: &str) -> Result<(u32, ComparisonType), Box<Error>> {
        // safe to unwrap as the regex is a compile-time constant.
        let re = Regex::new("^([/-]?)([0-7]+)$").unwrap();

        // have we been given a simple octal based string (e.g. /222)?
        if let Some(m) = re.captures(string_value) {
            // all these unwraps are safe because we checked the string in the regex above
            match u32::from_str_radix(m.get(2).unwrap().as_str(), 8) {
                Ok(val) => {
                    return Ok((val, m.get(1).unwrap().as_str().parse().unwrap()));
                }
                Err(e) => {
                    return Err(From::from(format!("Failed to parse -perm argument {}: {}",
                                                  m.get(2).unwrap().as_str(),
                                                  e)));
                }
            }

        }
        // no: so we've got a /u=rw,g=r form instead (or an invalid string).
        let mut p = Parser::new(string_value);
        for c in string_value.chars() {
            p.handle_char(&c)?;
        }
        Ok((p.bit_pattern, p.comparison_type))
    }
}

#[cfg(unix)]
pub struct PermMatcher {
    pattern: u32,
    comparison_type: ComparisonType,
}

#[cfg(not(unix))]
pub struct PermMatcher {}

impl PermMatcher {
    #[cfg(unix)]
    pub fn new(pattern: &str) -> Result<PermMatcher, Box<Error>> {
        let (bit_pattern, comparison_type) = parsing::parse(pattern)?;
        Ok(PermMatcher {
            pattern: bit_pattern,
            comparison_type: comparison_type,
        })
    }

    #[cfg(not(unix))]
    pub fn new(_dummy_pattern: &str) -> Result<PermMatcher, Box<Error>> {
        Err(From::from("Permission matching is not available on this platform"))
    }

    pub fn new_box(pattern: &str) -> Result<Box<Matcher>, Box<Error>> {
        Ok(Box::new(PermMatcher::new(pattern)?))
    }
}

impl Matcher for PermMatcher {
    #[cfg(unix)]
    fn matches(&self, file_info: &DirEntry, _: &mut MatcherIO) -> bool {
        use std::os::unix::fs::PermissionsExt;
        match file_info.metadata() {
            Ok(metadata) => {
                self.comparison_type.mode_bits_match(self.pattern, metadata.permissions().mode())
            }
            Err(e) => {
                writeln!(&mut stderr(),
                         "Error getting permissions for {}: {}",
                         file_info.path().to_string_lossy(),
                         e)
                    .unwrap();
                false
            }
        }
    }

    #[cfg(not(unix))]
    fn matches(&self, _dummy_file_info: &DirEntry, _: &mut MatcherIO) -> bool {
        writeln!(&mut stderr(),
                 "Permission matching not available on this platform!")
            .unwrap();
        return false;
    }
}


#[cfg(test)]
#[cfg(unix)]
mod tests {
    use find::matchers::Matcher;
    use find::matchers::tests::get_dir_entry_for;
    use find::tests::FakeDependencies;
    use super::*;
    use super::parsing;

    #[test]
    fn parsing_prefix() {
        assert_eq!(parsing::parse("u=rwx").unwrap(),
                   (0o700, ComparisonType::Exact));
        assert_eq!(parsing::parse("-u=rwx").unwrap(),
                   (0o700, ComparisonType::AtLeast));
        assert_eq!(parsing::parse("/u=rwx").unwrap(),
                   (0o700, ComparisonType::AnyOf));
        assert_eq!(parsing::parse("700").unwrap(),
                   (0o700, ComparisonType::Exact));
        assert_eq!(parsing::parse("-700").unwrap(),
                   (0o700, ComparisonType::AtLeast));
        assert_eq!(parsing::parse("/700").unwrap(),
                   (0o700, ComparisonType::AnyOf));
    }

    #[test]
    fn parsing_octal() {
        assert_eq!(parsing::parse("/1").unwrap(), (0o1, ComparisonType::AnyOf));
        assert_eq!(parsing::parse("/7777").unwrap(),
                   (0o7777, ComparisonType::AnyOf));
    }

    #[test]
    fn parsing_human_readable_individual_bits() {
        assert_eq!(parsing::parse("/").unwrap(), (0o0, ComparisonType::AnyOf));

        assert_eq!(parsing::parse("/u=r").unwrap(),
                   (0o400, ComparisonType::AnyOf));
        assert_eq!(parsing::parse("/u=w").unwrap(),
                   (0o200, ComparisonType::AnyOf));
        assert_eq!(parsing::parse("/u=x").unwrap(),
                   (0o100, ComparisonType::AnyOf));
        assert_eq!(parsing::parse("/g=r").unwrap(),
                   (0o40, ComparisonType::AnyOf));
        assert_eq!(parsing::parse("/g=w").unwrap(),
                   (0o20, ComparisonType::AnyOf));
        assert_eq!(parsing::parse("/g=x").unwrap(),
                   (0o10, ComparisonType::AnyOf));
        assert_eq!(parsing::parse("/o+r").unwrap(),
                   (0o4, ComparisonType::AnyOf));
        assert_eq!(parsing::parse("/o+w").unwrap(),
                   (0o2, ComparisonType::AnyOf));
        assert_eq!(parsing::parse("/o+x").unwrap(),
                   (0o1, ComparisonType::AnyOf));
        assert_eq!(parsing::parse("/a+r").unwrap(),
                   (0o444, ComparisonType::AnyOf));
        assert_eq!(parsing::parse("/a+w").unwrap(),
                   (0o222, ComparisonType::AnyOf));
        assert_eq!(parsing::parse("/a+x").unwrap(),
                   (0o111, ComparisonType::AnyOf));
    }

    #[test]
    fn parsing_human_readable_multiple_bits() {
        assert_eq!(parsing::parse("/u=rwx").unwrap(),
                   (0o700, ComparisonType::AnyOf));
        assert_eq!(parsing::parse("/a=rwx").unwrap(),
                   (0o777, ComparisonType::AnyOf));
    }

    #[test]
    fn parsing_human_readable_multiple_categories() {
        assert_eq!(parsing::parse("/u=rwx,g=rx,o+r").unwrap(),
                   (0o754, ComparisonType::AnyOf));
        assert_eq!(parsing::parse("/u=rwx,g=rx,o+r,a+w").unwrap(),
                   (0o776, ComparisonType::AnyOf));
        assert_eq!(parsing::parse("/ug=rwx,o+r").unwrap(),
                   (0o774, ComparisonType::AnyOf));
    }

    #[test]
    fn parsing_human_readable_set_id_bits() {
        assert_eq!(parsing::parse("/u=s").unwrap(),
                   (0o4000, ComparisonType::AnyOf));
        assert_eq!(parsing::parse("/g=s").unwrap(),
                   (0o2000, ComparisonType::AnyOf));
        assert_eq!(parsing::parse("/ug=s").unwrap(),
                   (0o6000, ComparisonType::AnyOf));
        assert_eq!(parsing::parse("/o=s").unwrap(),
                   (0o0000, ComparisonType::AnyOf));
    }

    #[test]
    fn parsing_human_readable_sticky_bit() {
        assert_eq!(parsing::parse("/u=t").unwrap(),
                   (0o1000, ComparisonType::AnyOf));
        assert_eq!(parsing::parse("/g=t").unwrap(),
                   (0o1000, ComparisonType::AnyOf));
        assert_eq!(parsing::parse("/o=t").unwrap(),
                   (0o1000, ComparisonType::AnyOf));
    }


    #[test]
    fn parsing_fails() {
        assert!(parsing::parse("+u=rwx,g=rx,o+r").is_err(),
                "invalid prefix should fail");
        assert!(parsing::parse("urwx,g=rx,o+r").is_err(),
                "missing equals should fail");
        assert!(parsing::parse("d=rwx,g=rx,o+r").is_err(),
                "invalid category should fail");
        assert!(parsing::parse("u=dwx,g=rx,o+r").is_err(),
                "invalid permission bit should fail");
        assert!(parsing::parse("u=rwxg=rx,o+r").is_err(),
                "missing comma should fail");
        assert!(parsing::parse("u_rwx,g=rx,o+r").is_err(),
                "invalid category/permissoin spearator should fail");
        assert!(parsing::parse("77777777777777").is_err(),
                "overflowing octal value should fail");
    }

    #[test]
    fn comparison_type_matching() {
        let c = ComparisonType::Exact;
        assert!(c.mode_bits_match(0, 0),
                "Exact: only 0 should match if pattern is 0");
        assert!(!c.mode_bits_match(0, 0o444),
                "Exact: only 0 should match if pattern is 0");
        assert!(c.mode_bits_match(0o444, 0o444),
                "Exact: identical bits should match");
        assert!(!c.mode_bits_match(0o444, 0o777),
                "Exact: non-identical bits should fail");
        assert!(c.mode_bits_match(0o444, 0o70444),
                "Exact:high-end bits should be ignored");



        let c = ComparisonType::AtLeast;
        assert!(c.mode_bits_match(0, 0),
                "AtLeast: anything should match if pattern is 0");
        assert!(c.mode_bits_match(0, 0o444),
                "AtLeast: anything should match if pattern is 0");
        assert!(c.mode_bits_match(0o444, 0o777),
                "AtLeast: identical bits should match");
        assert!(c.mode_bits_match(0o444, 0o777),
                "AtLeast: extra bits should match");
        assert!(!c.mode_bits_match(0o444, 0o700),
                "AtLeast: missing bits should fail");
        assert!(c.mode_bits_match(0o444, 0o70444),
                "AtLeast: high-end bits should be ignored");

        let c = ComparisonType::AnyOf;
        assert!(c.mode_bits_match(0, 0),
                "AnyOf: anything should match if pattern is 0");
        assert!(c.mode_bits_match(0, 0o444),
                "AnyOf: anything should match if pattern is 0");
        assert!(c.mode_bits_match(0o444, 0o777),
                "AnyOf: identical bits should match");
        assert!(c.mode_bits_match(0o444, 0o777),
                "AnyOf: extra bits should match");
        assert!(c.mode_bits_match(0o777, 0o001),
                "AnyOf: anything should match as long as it has one bit in common");
        assert!(!c.mode_bits_match(0o010, 0o001),
                "AnyOf: no matching bits shouldn't match");
        assert!(c.mode_bits_match(0o444, 0o70444),
                "AnyOf: high-end bits should be ignored");
    }

    #[test]
    fn perm_matches() {
        let file_info = get_dir_entry_for("test_data/simple", "abbbc");
        let deps = FakeDependencies::new();

        let matcher = PermMatcher::new("-u+r").unwrap();
        assert!(matcher.matches(&file_info, &mut deps.new_matcher_io()),
                "user-readable pattern should match file");

        let matcher = PermMatcher::new("-u+x").unwrap();
        assert!(!matcher.matches(&file_info, &mut deps.new_matcher_io()),
                "user-executable pattern should not match file");
    }
}
