// Copyright 2022 Tavian Barnes
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::sync::LazyLock;

use onig::{Regex, RegexOptions, Syntax, SyntaxBehavior};

/// Compile a glob's BRE with backslash escapes enabled inside bracket expressions.
fn parse_bre(expr: &str, options: RegexOptions) -> Result<Regex, onig::Error> {
    static SYNTAX: LazyLock<Syntax> = LazyLock::new(|| {
        let mut syntax = *Syntax::posix_basic();
        syntax.enable_behavior(SyntaxBehavior::SYNTAX_BEHAVIOR_BACKSLASH_ESCAPE_IN_CC);
        syntax
    });
    Regex::with_options(expr, SYNTAX.options() | options, &SYNTAX)
}

/// Push a literal character outside a bracket expression, escaping it if necessary.
fn regex_push_literal(regex: &mut String, ch: char) {
    // https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/V1_chap09.html#tag_09_03_03
    if matches!(ch, '.' | '[' | '\\' | '*' | '^' | '$') {
        regex.push('\\');
    }
    regex.push(ch);
}

/// Extracts a bracket expression from a glob.
fn extract_bracket_expr(mut pattern: &str) -> Option<(String, &str)> {
    // https://pubs.opengroup.org/onlinepubs/9699919799/utilities/V3_chap02.html#tag_18_13_01
    //
    //     If an open bracket introduces a bracket expression as in XBD RE Bracket Expression,
    //     except that the <exclamation-mark> character ( '!' ) shall replace the <circumflex>
    //     character ( '^' ) in its role in a non-matching list in the regular expression notation,
    //     it shall introduce a pattern bracket expression. A bracket expression starting with an
    //     unquoted <circumflex> character produces unspecified results. Otherwise, '[' shall match
    //     the character itself.
    //
    // To check for valid bracket expressions, we scan for the closing bracket and
    // attempt to parse that segment as a regex.  If that fails, we treat the '['
    // literally.

    let mut expr = "[".to_string();

    // https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/V1_chap09.html#tag_09_03_05
    //
    //     3. A non-matching list expression begins with a <circumflex> ( '^' ) ...
    //
    // (but in a glob, '!' is used instead of '^')
    if let Some(rest) = pattern.strip_prefix('!') {
        expr.push('^');
        pattern = rest;
    }

    // https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/V1_chap09.html#tag_09_03_05
    //
    //     1. ... The <right-square-bracket> ( ']' ) shall lose its special meaning and represent
    //        itself in a bracket expression if it occurs first in the list (after an initial
    //        <circumflex> ( '^' ), if any).
    if let Some(rest) = pattern.strip_prefix(']') {
        expr.push(']');
        pattern = rest;
    }

    let mut chars = pattern.chars();
    loop {
        match chars.next()? {
            '\\' => {
                let literal = chars.next()?;
                // A glob escape quotes one character. Keep the backslash only
                // for bracket metacharacters so e.g. \d matches a literal d.
                if matches!(literal, '[' | ']' | '-' | '^' | '\\') {
                    expr.push('\\');
                }
                expr.push(literal);
            }
            '[' if chars.as_str().starts_with(['.', '=', ':']) => {
                // https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/V1_chap09.html#tag_09_03_05
                //
                //     4. A collating symbol is a collating element enclosed within bracket-period
                //        ( "[." and ".]" ) delimiters. ...
                //
                //     5. An equivalence class expression shall ... be expressed by enclosing any
                //        one of the collating elements in the equivalence class within bracket-
                //        equal ( "[=" and "=]" ) delimiters.
                //
                //     6. ...  A character class expression is expressed as a character class name
                //        enclosed within bracket- <colon> ( "[:" and ":]" ) delimiters.
                let delim = chars.next()?;
                let closer = format!("{delim}]");
                let (name, rest) = chars.as_str().split_once(&closer)?;
                if delim == ':' && name == "digit" {
                    // POSIX limits [:digit:] to 0-9 in every locale, but
                    // Oniguruma uses its Unicode table (fullwidth and
                    // Arabic-Indic digits too), so spell out the range.
                    expr.push_str("0-9");
                } else {
                    expr.push('[');
                    expr.push(delim);
                    expr.push_str(name);
                    expr.push_str(&closer);
                }
                chars = rest.chars();
            }
            '[' => expr.push_str(r"\["),
            ']' => {
                // https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/V1_chap09.html#tag_09_03_05
                //
                //     1. ... The <right-square-bracket> ( ']' ) shall ... terminate the bracket
                //        expression, unless it appears in a collating symbol (such as "[.].]" ) or is
                //        the ending <right-square-bracket> for a collating symbol, equivalence class,
                //        or character class.
                expr.push(']');
                break;
            }
            ch => expr.push(ch),
        }
    }

    if parse_bre(&expr, RegexOptions::REGEX_OPTION_NONE).is_ok() {
        Some((expr, chars.as_str()))
    } else {
        None
    }
}

/// Translate an fnmatch-style glob to a POSIX basic regex with backslash escapes
/// enabled inside bracket expressions.
fn glob_to_regex(pattern: &str) -> Option<String> {
    let mut regex = String::new();

    let mut chars = pattern.chars();
    while let Some(ch) = chars.next() {
        // https://pubs.opengroup.org/onlinepubs/9699919799/utilities/V3_chap02.html#tag_18_13
        match ch {
            '?' => regex.push('.'),
            '*' => regex.push_str(".*"),
            '\\' => {
                // https://pubs.opengroup.org/onlinepubs/9699919799/functions/fnmatch.html
                //
                //     If pattern ends with an unescaped <backslash>, fnmatch() shall return a
                //     non-zero value (indicating either no match or an error).
                //
                // Most implementations return FNM_NOMATCH in this case, so create a pattern that
                // never matches.
                let ch = chars.next()?;
                regex_push_literal(&mut regex, ch);
            }
            '[' => {
                if let Some((expr, rest)) = extract_bracket_expr(chars.as_str()) {
                    regex.push_str(&expr);
                    chars = rest.chars();
                } else {
                    regex_push_literal(&mut regex, ch);
                }
            }
            _ => regex_push_literal(&mut regex, ch),
        }
    }

    Some(regex)
}

/// An fnmatch()-style glob matcher.
pub struct Pattern {
    regex: Option<Regex>,
}

impl Pattern {
    /// Parse an fnmatch()-style glob.
    pub fn new(pattern: &str, caseless: bool) -> Self {
        let options = if caseless {
            RegexOptions::REGEX_OPTION_IGNORECASE
        } else {
            RegexOptions::REGEX_OPTION_NONE
        };

        // As long as glob_to_regex() is correct, this should never fail
        let regex = glob_to_regex(pattern).map(|r| parse_bre(&r, options).unwrap());
        Self { regex }
    }

    /// Test if this pattern matches a string.
    pub fn matches(&self, string: &str) -> bool {
        self.regex.as_ref().is_some_and(|r| r.is_match(string))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[track_caller]
    fn assert_glob_regex(glob: &str, regex: &str) {
        assert_eq!(glob_to_regex(glob).as_deref(), Some(regex));
    }

    #[test]
    fn literals() {
        assert_glob_regex(r"foo.bar", r"foo\.bar");
    }

    #[test]
    fn regex_special() {
        assert_glob_regex(r"^foo.bar$", r"\^foo\.bar\$");
    }

    #[test]
    fn wildcards() {
        assert_glob_regex(r"foo?bar*baz", r"foo.bar.*baz");
    }

    #[test]
    fn escapes() {
        assert_glob_regex(r"fo\o\?bar\*baz\\", r"foo?bar\*baz\\");
    }

    #[test]
    fn valid_brackets() {
        assert_glob_regex(r"foo[bar][!baz]", r"foo[bar][^baz]");
    }

    #[rstest]
    #[case(r"[a\]]", &["a", "]"])]
    #[case(r"[!a\]]", &["[", r"\"])]
    #[case(r"[[\]]", &["]", "["])]
    #[case(r"[\[-\]]", &["]", "[", r"\"])]
    #[case(r"[a\\]]", &["a]", r"\]"])]
    #[case(r"[a\]", &["[a]"])]
    #[case(r"[\a]", &["a"])]
    fn escaped_brackets(#[case] glob: &str, #[case] expected: &[&str]) {
        let pattern = Pattern::new(glob, false);
        let matches: Vec<_> = ["a", "]", "a]", "[", r"\", r"\]", "[a]"]
            .into_iter()
            .filter(|name| pattern.matches(name))
            .collect();
        assert_eq!(matches, expected);
    }

    #[test]
    fn complex_brackets() {
        // A doubled backslash leaves the following collating symbol `[.].]` intact.
        assert_glob_regex(
            r"[!]!.*[\\[.].][=]=][:space:]-]",
            r"[^]!.*\[\\[.].][=]=][:space:]-]",
        );
    }

    #[test]
    fn digit_class_is_ascii_range() {
        assert_glob_regex(r"[[:digit:]]*", r"[0-9].*");
        assert_glob_regex(r"[![:digit:]a]", r"[^0-9a]");
    }

    #[rstest]
    #[case("7", true)]
    #[case("\u{ff17}", false)] // FULLWIDTH DIGIT SEVEN
    #[case("\u{0667}", false)] // ARABIC-INDIC DIGIT SEVEN
    fn digit_class_is_ascii_only(#[case] name: &str, #[case] expected: bool) {
        assert_eq!(Pattern::new("[[:digit:]]", false).matches(name), expected);
        assert_eq!(Pattern::new("[![:digit:]]", false).matches(name), !expected);
    }

    #[test]
    fn invalid_brackets() {
        assert_glob_regex(r"foo[bar[!baz", r"foo\[bar\[!baz");
    }

    #[test]
    fn malformed_posix_class_with_multibyte_char() {
        for pat in ["[[:]é", "[[:a]é", "[[.]é", "[[:é]", "[[=]😀"] {
            assert!(
                glob_to_regex(pat).is_some(),
                "panicked or rejected: {pat:?}"
            );
        }
    }

    #[test]
    fn incomplete_escape() {
        assert_eq!(glob_to_regex(r"foo\"), None);
    }

    #[test]
    fn pattern_matches() {
        assert!(Pattern::new(r"foo*bar", false).matches("foo--bar"));

        assert!(!Pattern::new(r"foo*bar", false).matches("bar--foo"));
    }

    #[test]
    fn caseless_matches() {
        assert!(Pattern::new(r"foo*BAR", true).matches("FOO--bar"));

        assert!(!Pattern::new(r"foo*BAR", true).matches("BAR--foo"));
    }

    #[test]
    fn incomplete_escape_matches() {
        assert!(!Pattern::new(r"foo\", false).matches("\n"));
    }
}
