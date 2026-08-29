//! # Search error
//!
//! What a search query fails to parse with.

use std::fmt;

use chumsky::error::Rich;

/// A search query that did not parse.
#[derive(Debug)]
pub enum Error {
    /// The chumsky diagnostics, and the query they were raised over.
    ParseError(Vec<Rich<'static, char>>, String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self::ParseError(_, query) = self;
        write!(f, "cannot parse search emails query `{query}`")
    }
}

impl std::error::Error for Error {}
