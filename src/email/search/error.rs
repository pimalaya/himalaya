use std::fmt;

use chumsky::error::Rich;

/// Search-query parse failure, carrying the rich `chumsky` errors (for
/// pretty diagnostics) alongside the original query string.
#[derive(Debug)]
pub enum Error {
    ParseError(Vec<Rich<'static, char>>, String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self::ParseError(_, query) = self;
        write!(f, "cannot parse search emails query `{query}`")
    }
}

impl std::error::Error for Error {}
