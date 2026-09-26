// This file is part of the uutils findutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Expression parse errors that remember which argument they refer to.
//! `Display` is the plain GNU message.

use std::error::Error;

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
}

impl ParseError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            arg_index: None,
            label: None,
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

    pub fn arg_index(&self) -> Option<usize> {
        self.arg_index
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_is_the_bare_message() {
        let error = ParseError::new("unknown predicate `-zap'")
            .at(3)
            .with_label("not a known predicate");
        assert_eq!(error.to_string(), "unknown predicate `-zap'");
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
}
