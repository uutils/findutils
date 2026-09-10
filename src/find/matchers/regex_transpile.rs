// This file is part of the uutils findutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::error::Error;

use fancy_regex::{Regex, RegexBuilder};

use super::regex::RegexType;

/// Swap BRE escapes to fancy-regex syntax (`\(` → `(`, `+` → `\+`, etc.).
fn transpile_bre_like(pattern: &str) -> String {
    let mut output = String::with_capacity(pattern.len() * 2);
    let mut chars = pattern.chars().peekable();
    let mut in_bracket = false;

    while let Some(ch) = chars.next() {
        if in_bracket {
            if ch == ']' && output.ends_with(|c| c != '\\' && c != '[' && c != '^') {
                in_bracket = false;
            }
            output.push(ch);
            continue;
        } else if ch == '[' {
            in_bracket = true;
            output.push(ch);
            continue;
        } else if ch != '\\' {
            match ch {
                '+' | '?' | '|' | '(' | ')' | '{' | '}' => {
                    output.push('\\');
                    output.push(ch);
                }
                '^' => {
                    if output.is_empty() || output.ends_with('\\') {
                        output.push('^');
                    } else {
                        output.push_str(r"\^");
                    }
                }
                '$' => {
                    if chars.peek().is_none() || chars.peek() == Some(&'\\') {
                        output.push('$');
                    } else {
                        output.push_str(r"\$");
                    }
                }
                _ => output.push(ch),
            }
            continue;
        }

        match chars.next() {
            Some(c) if "(){}|+?".contains(c) => output.push(c),
            Some(c) => {
                output.push('\\');
                output.push(c);
            }
            None => {
                output.push('\\');
            }
        }
    }

    output
}

/// Translate Emacs regex syntax to fancy-regex.
///
/// Emacs uses `\(` `\)` `\{m,n\}` (BRE-style escapes) for groups and
/// intervals, but ERE-style `+` `?` `|` for quantifiers and alternation.
fn transpile_emacs(pattern: &str) -> String {
    let mut output = String::with_capacity(pattern.len() + 8);
    let mut chars = pattern.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            if "(){}".contains(ch) {
                output.push('\\');
            }
            output.push(ch);
            continue;
        }

        match chars.next() {
            Some(c) if "(){}|+?".contains(c) => output.push(c),
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
        RegexType::PosixBasic | RegexType::Grep => transpile_bre_like(pattern),
        RegexType::Emacs => transpile_emacs(pattern),
        RegexType::PosixExtended => pattern.to_owned(),
    };

    let mut builder = RegexBuilder::new(&transpiled);
    builder.oniguruma_mode(true);
    if ignore_case {
        builder.case_insensitive(true);
    }
    builder.build().map_err(Into::into)
}

/// Check if a POSIX BRE pattern is syntactically valid.
pub fn is_valid_bre(pattern: &str) -> bool {
    let transpiled = transpile_bre_like(pattern);
    Regex::new(&transpiled).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bre_groups() {
        assert_eq!(transpile_bre_like(r"foo\(bar\)baz"), r"foo(bar)baz");
    }

    #[test]
    fn bre_intervals() {
        assert_eq!(transpile_bre_like(r"ab\{1,3\}c"), "ab{1,3}c");
    }

    #[test]
    fn bre_alternation() {
        assert_eq!(transpile_bre_like(r"foo\|bar"), "foo|bar");
    }

    #[test]
    fn bre_literal_specials() {
        assert_eq!(
            transpile_bre_like("a+b?c|d(e)f{g}"),
            r"a\+b\?c\|d\(e\)f\{g\}"
        );
    }

    #[test]
    fn bre_anchors() {
        assert_eq!(transpile_bre_like("^foo$"), "^foo$");
        assert_eq!(transpile_bre_like("a^b$c"), r"a\^b\$c");
    }

    #[test]
    fn bre_backrefs() {
        assert_eq!(transpile_bre_like(r"\(foo\)\1"), r"(foo)\1");
    }

    #[test]
    fn bre_escapes_preserved() {
        assert_eq!(transpile_bre_like(r"\.\*\\w"), r"\.\*\\w");
    }

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
}
