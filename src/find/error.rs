// This file is part of the uutils findutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Expression parse errors that remember which argument they refer to.
//! `Display` is the plain GNU message.

use std::error::Error;

use uucore::diagnostics::Snapshot;

/// A command-line expression error that can point at the argument that caused it.
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct ParseError {
    message: String,
    /// Index of the offending argument, relative to the argument slice this
    /// error was created in. [`ParseError::shift`] moves it along at each
    /// parsing-layer boundary until it indexes the process's full argv.
    arg_index: Option<usize>,
    label: Option<String>,
    help: Option<String>,
}

impl ParseError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            arg_index: None,
            label: None,
            help: None,
        }
    }

    /// Records which argument the error refers to.
    pub fn at(mut self, arg_index: usize) -> Self {
        self.arg_index = Some(arg_index);
        self
    }

    /// Sets the text shown under the underlined argument.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets a suggestion shown below the report.
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn arg_index(&self) -> Option<usize> {
        self.arg_index
    }

    pub fn help(&self) -> Option<&str> {
        self.help.as_deref()
    }

    /// Adds `by` to the argument index of a `ParseError`, for callers that
    /// parsed a sub-slice of argv. Other errors pass through untouched.
    pub fn shift(err: Box<dyn Error>, by: usize) -> Box<dyn Error> {
        match err.downcast::<Self>() {
            Ok(mut parse_error) => {
                if let Some(index) = parse_error.arg_index {
                    parse_error.arg_index = Some(index + by);
                }
                parse_error
            }
            Err(other) => other,
        }
    }

    /// Draws the error under the offending argument on stderr, headed by the
    /// plain message line. Returns `false`, having printed nothing, when the
    /// error names no argument.
    pub fn render(&self, argv: &[&str]) -> bool {
        let Some(arg_index) = self.arg_index else {
            return false;
        };
        // uucore translates the "Help:" label from its own embedded strings;
        // without a localizer it prints the raw message id instead. Done here
        // so that only a drawn report pays for it.
        let _ = uucore::locale::setup_localization("find");
        Snapshot::with_program(argv).render(
            arg_index,
            &self.message,
            self.label.as_deref(),
            self.help.as_deref(),
        )
    }
}

/// Returns the entry of `candidates` closest to `input`, if one is close enough
/// to be a plausible typo.
pub fn closest_match<'a>(
    input: &str,
    candidates: impl IntoIterator<Item = &'a str>,
) -> Option<&'a str> {
    // Allow one edit for short predicates and more for longer ones, but never
    // so many that unrelated names start matching. Two edits is the useful
    // minimum for anything but the shortest names, since a swapped pair of
    // letters already costs that much.
    let input_length = input.chars().count();
    let max_distance = match input_length {
        0..=3 => 1,
        4..=7 => 2,
        _ => 3,
    };

    candidates
        .into_iter()
        .map(|candidate| (edit_distance(input, candidate), candidate))
        // A suggestion is only worth making if more of what was typed survives
        // it than not. Without this, every two-character candidate sits one
        // edit from every other, and `-H` -- a real option a user is likely to
        // misplace after the path operand -- would be "corrected" to whichever
        // short predicate happens to come first in the candidate list.
        .filter(|(distance, _)| *distance <= max_distance && distance * 2 < input_length)
        .min_by_key(|(distance, candidate)| (*distance, candidate.len()))
        .map(|(_, candidate)| candidate)
}

/// Levenshtein distance between two strings, counting characters rather than
/// bytes.
fn edit_distance(left: &str, right: &str) -> usize {
    let right_chars: Vec<char> = right.chars().collect();
    // Distances from the empty prefix of `left` to each prefix of `right`.
    let mut previous_row: Vec<usize> = (0..=right_chars.len()).collect();
    let mut current_row = vec![0; right_chars.len() + 1];

    for (left_index, left_char) in left.chars().enumerate() {
        current_row[0] = left_index + 1;
        for (right_index, &right_char) in right_chars.iter().enumerate() {
            let substitution_cost = usize::from(left_char != right_char);
            current_row[right_index + 1] = (current_row[right_index] + 1)
                .min(previous_row[right_index + 1] + 1)
                .min(previous_row[right_index] + substitution_cost);
        }
        std::mem::swap(&mut previous_row, &mut current_row);
    }

    previous_row[right_chars.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_is_the_bare_message() {
        let error = ParseError::new("unknown predicate `-zap'")
            .at(3)
            .with_label("not a known predicate")
            .with_help("did you mean `-zip'?");
        assert_eq!(error.to_string(), "unknown predicate `-zap'");
    }

    #[test]
    fn render_without_an_index_draws_nothing() {
        // Nothing to point at: the caller has to print the plain line.
        let error = ParseError::new("something went wrong");
        assert!(!error.render(&["find", "/srv"]));
    }

    #[test]
    fn shift_moves_the_index_and_composes() {
        let error: Box<dyn Error> = Box::new(ParseError::new("oops").at(2));
        let error = ParseError::shift(ParseError::shift(error, 3), 1);
        let shifted = error.downcast_ref::<ParseError>().unwrap();
        assert_eq!(shifted.arg_index(), Some(6));
    }

    #[test]
    fn shift_leaves_indexless_and_foreign_errors_alone() {
        let indexless: Box<dyn Error> = Box::new(ParseError::new("oops"));
        let indexless = ParseError::shift(indexless, 4);
        assert_eq!(
            indexless.downcast_ref::<ParseError>().unwrap().arg_index(),
            None
        );

        let foreign: Box<dyn Error> = From::from("plain message");
        let foreign = ParseError::shift(foreign, 4);
        assert_eq!(foreign.to_string(), "plain message");
        assert!(foreign.downcast_ref::<ParseError>().is_none());
    }

    #[test]
    fn edit_distance_counts_single_edits() {
        assert_eq!(edit_distance("", ""), 0);
        assert_eq!(edit_distance("-name", "-name"), 0);
        assert_eq!(edit_distance("-nmae", "-name"), 2);
        assert_eq!(edit_distance("-nam", "-name"), 1);
        assert_eq!(edit_distance("", "-name"), 5);
        assert_eq!(edit_distance("-name", ""), 5);
    }

    #[test]
    fn closest_match_finds_plausible_typos() {
        let candidates = ["-name", "-newer", "-nogroup", "-print"];
        assert_eq!(closest_match("-nmae", candidates), Some("-name"));
        assert_eq!(closest_match("-printt", candidates), Some("-print"));
        assert_eq!(closest_match("-nogruop", candidates), Some("-nogroup"));
    }

    #[test]
    fn closest_match_rejects_short_input() {
        // `-H` is one edit from any other two-character name, so a suggestion
        // here would be a coin toss dressed up as advice.
        let candidates = ["-a", "-o", "-ls", "-name"];
        assert_eq!(closest_match("-H", candidates), None);
        assert_eq!(closest_match("-P", candidates), None);
        // Three characters leave a majority intact after a single edit, so a
        // suggestion is fair game again.
        assert_eq!(closest_match("-lz", candidates), Some("-ls"));
    }

    #[test]
    fn closest_match_rejects_distant_input() {
        let candidates = ["-name", "-newer", "-print"];
        assert_eq!(closest_match("-zzzzzzzzzz", candidates), None);
        assert_eq!(closest_match("-xyz", candidates), None);
    }
}
