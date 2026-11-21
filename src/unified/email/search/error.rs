//! # Search error
//!
//! The refusal of a clause a backend cannot search.

use pimalaya_cli::error::CodedError;

use crate::{error::ErrorCode, unified::email::search::filter::query::SearchEmailsFilterQuery};

/// Refuses a clause the backend cannot search, quoting it as the query
/// language writes it and saying why, under the code `search-unsupported`.
///
/// Raised while the query is translated, before any request, so nothing
/// is searched and no clause is silently dropped.
pub fn unsupported(clause: &SearchEmailsFilterQuery, reason: &str) -> anyhow::Error {
    let message = format!("Cannot search `{clause}`: {reason}");
    CodedError::new(ErrorCode::SearchUnsupported, message).into()
}
