// This file is part of the uutils findutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::error::Error;

use fancy_regex::{Regex, RegexBuilder};

use super::{bre_to_ere::bre_to_ere, regex::RegexType};

/// Translate Emacs regex syntax to fancy-regex.
///
/// Emacs uses `\(` `\)` `\{m,n\}` `\|` (BRE-style escapes) for groups,
/// intervals, and alternation, but ERE-style `+` `?` for quantifiers.
fn transpile_emacs(pattern: &str) -> String {
    let mut output = String::with_capacity(pattern.len() + 8);
    let mut chars = pattern.chars().peekable();
    let mut in_bracket = false;

    while let Some(ch) = chars.next() {
        if in_bracket {
            if ch == ']' && !output.ends_with(['[', '^']) {
                in_bracket = false;
            }
            output.push(ch);
            continue;
        }

        if ch != '\\' {
            if ch == '[' {
                in_bracket = true;
            } else if "(){}|".contains(ch) {
                output.push('\\');
            }
            output.push(ch);
            continue;
        }

        match chars.next() {
            Some(c) if "(){}|".contains(c) => output.push(c),
            Some(c) => {
                output.push('\\');
                output.push(c);
            }
            None => output.push('\\'),
        }
    }

    output
}
/// Compile a regex pattern with the given flavor into a `fancy_regex::Regex`.
pub fn compile(
    pattern: &str,
    regex_type: RegexType,
    ignore_case: bool,
) -> Result<Regex, Box<dyn Error>> {
    let transpiled = match regex_type {
        RegexType::PosixBasic | RegexType::Grep => bre_to_ere(pattern, false)?,
        RegexType::Emacs => transpile_emacs(pattern),
        RegexType::PosixExtended => pattern.to_owned(),
    };

    let mut builder = RegexBuilder::new(&transpiled);
    builder.oniguruma_mode(true);
    builder.leftmost_longest(true);
    builder.seek(true);
    if ignore_case {
        builder.case_insensitive(true);
    }
    builder.build().map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ere_pass_through() {
        let re = compile(r".*/ab{1,3}c", RegexType::PosixExtended, true).unwrap();
        assert!(re.is_match("/abbbc").unwrap());
        assert!(re.is_match("/abbc").unwrap());
    }

    #[test]
    fn bre_compile_intervals() {
        let re = compile(r".*/ab\{1,3\}c", RegexType::PosixBasic, true).unwrap();
        assert!(re.is_match("/abbbc").unwrap());
        assert!(re.is_match("/abbc").unwrap());
    }

    #[test]
    fn bre_compile_backref() {
        let re = compile(r"\(a\)\1", RegexType::PosixBasic, false).unwrap();
        assert!(re.is_match("aa").unwrap());
        assert!(!re.is_match("ab").unwrap());
    }

    #[test]
    fn test_leftmost_longest() {
        let re = compile(r"a\|ab", RegexType::PosixBasic, false).unwrap();
        let m = re.find("ab").unwrap().unwrap();
        assert_eq!(m.as_str(), "ab");
    }

    #[test]
    fn test_emacs_regex_syntax() {
        // Bare | is literal in Emacs, \| is alternation
        let re_literal_pipe = compile(r"^a|b$", RegexType::Emacs, false).unwrap();
        assert!(re_literal_pipe.is_match("a|b").unwrap());
        assert!(!re_literal_pipe.is_match("a").unwrap());
        assert!(!re_literal_pipe.is_match("b").unwrap());

        let re_alt = compile(r"^\(a\|b\)$", RegexType::Emacs, false).unwrap();
        assert!(re_alt.is_match("a").unwrap());
        assert!(re_alt.is_match("b").unwrap());
        assert!(!re_alt.is_match("a|b").unwrap());

        // Bare +/? are quantifiers, \+/\? are literals
        let re_quant = compile(r"^a+b$", RegexType::Emacs, false).unwrap();
        assert!(re_quant.is_match("ab").unwrap());
        assert!(re_quant.is_match("aaab").unwrap());

        let re_lit_plus = compile(r"^a\+b$", RegexType::Emacs, false).unwrap();
        assert!(re_lit_plus.is_match("a+b").unwrap());
        assert!(!re_lit_plus.is_match("ab").unwrap());

        let re_lit_qmark = compile(r"^a\?b$", RegexType::Emacs, false).unwrap();
        assert!(re_lit_qmark.is_match("a?b").unwrap());
        assert!(!re_lit_qmark.is_match("ab").unwrap());

        // Bracket expressions
        let re_bracket = compile(r"^[a|b]$", RegexType::Emacs, false).unwrap();
        assert!(re_bracket.is_match("|").unwrap());
        assert!(re_bracket.is_match("a").unwrap());
        assert!(!re_bracket.is_match(r"\").unwrap());
    }

    #[test]
    fn test_posix_character_classes() {
        let re = compile(r"[[:alpha:]]*", RegexType::PosixBasic, false).unwrap();
        assert!(re.is_match("hello").unwrap());
        assert!(re.is_match("é").unwrap());
        let re_exact = compile(r"^[[:alpha:]]$", RegexType::PosixBasic, false).unwrap();
        assert!(re_exact.is_match("é").unwrap());
        assert!(!re_exact.is_match("1").unwrap());
    }
}
