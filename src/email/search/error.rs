//! # Search error
//!
//! What a search query fails to parse with, and the refusal of a clause a
//! backend cannot search.

use std::fmt;

use chumsky::error::Rich;
#[cfg(any(feature = "imap", feature = "gmail", feature = "msgraph"))]
use pimalaya_cli::error::CodedError;

#[cfg(any(feature = "imap", feature = "gmail", feature = "msgraph"))]
use crate::{email::search::filter::query::SearchEmailsFilterQuery, error::ErrorCode};

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

/// Refuses a clause the backend cannot search, quoting it as written and
/// saying why, under the code `search-unsupported`.
///
/// Raised while the query is translated, before any request, so nothing
/// is searched and no clause is silently dropped.
#[cfg(any(feature = "imap", feature = "gmail", feature = "msgraph"))]
pub fn unsupported(clause: &SearchEmailsFilterQuery, reason: &str) -> anyhow::Error {
    let message = format!("Cannot search `{clause}`: {reason}");
    CodedError::new(ErrorCode::SearchUnsupported, message).into()
}
