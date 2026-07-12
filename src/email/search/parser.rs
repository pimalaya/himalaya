//! # Search emails query string parser
//!
//! Entry point for parsing a full search emails query from a string
//! slice. Defers to [`super::filter::parser`] and
//! [`super::sort::parser`] for the actual grammar. Based on [`chumsky`].

use chumsky::{Parser, error::Rich, extra};

use crate::email::search::{
    error::Error,
    filter::{self, query::SearchEmailsFilterQuery},
    query::SearchEmailsQuery,
    sort::{self, query::SearchEmailsSorter},
};

/// Alias for a rich [`chumsky`] error for better diagnostics.
pub type ParserError<'a> = extra::Err<Rich<'a, char>>;

/// Parses `input` into a [`SearchEmailsQuery`].
///
/// Because of the recursive nature of [`SearchEmailsFilterQuery`], the
/// string is split around the `"order by"` keyword and the two halves
/// are parsed separately. A query string can be filter-only, sort-only,
/// or filter then sort.
pub fn parse(input: impl AsRef<str>) -> Result<SearchEmailsQuery, Error> {
    let input = input.as_ref().trim();

    if let Some((filters_input, sorters_input)) = input.rsplit_once("order by") {
        if filters_input.trim().is_empty() {
            let filter = None;
            let sort = parse_sort(sorters_input).map(Some)?;
            Ok(SearchEmailsQuery { filter, sort })
        } else {
            let filter = parse_filter(filters_input).map(Some)?;
            let sort = parse_sort(sorters_input).map(Some)?;
            Ok(SearchEmailsQuery { filter, sort })
        }
    } else {
        let filter = parse_filter(input).map(Some)?;
        let sort = None;
        Ok(SearchEmailsQuery { filter, sort })
    }
}

/// Parses `input` into a [`SearchEmailsFilterQuery`].
pub fn parse_filter(input: impl AsRef<str>) -> Result<SearchEmailsFilterQuery, Error> {
    let input = input.as_ref().trim();

    filter::parser::query()
        .parse(input)
        .into_result()
        .map_err(|errs| {
            let errs = errs
                .into_iter()
                .map(|err| err.clone().into_owned())
                .collect();
            Error::ParseError(errs, String::from(input))
        })
}

/// Parses `input` into a list of [`SearchEmailsSorter`].
pub fn parse_sort(input: impl AsRef<str>) -> Result<Vec<SearchEmailsSorter>, Error> {
    let input = input.as_ref().trim();

    sort::parser::query()
        .parse(input)
        .into_result()
        .map_err(|errs| {
            let errs = errs
                .into_iter()
                .map(|err| err.clone().into_owned())
                .collect();
            Error::ParseError(errs, String::from(input))
        })
}
