//! # Gmail search
//!
//! Bridges the shared search query onto the Gmail `messages.list` `q`
//! syntax, the same one the Gmail search box speaks.
//!
//! The filter half becomes one `q` string: leaf conditions map to
//! operators (`from alice` becomes `from:alice`), conjunctions flatten
//! into whitespace, disjunctions into brace groups, and negations expand
//! through De Morgan so a `-` only ever prefixes a leaf, the shape Gmail
//! matches reliably. Two conditions have no Gmail counterpart and bail
//! rather than return wrong hits: `flag answered`, replies being
//! tracked in no searchable way, and flags outside the set below.
//!
//! The sort half has no server-side counterpart, `messages.list`
//! ordering being fixed newest-first, so it orders the fetched page
//! client-side. The caveat: sorting a page is not sorting the result
//! set, page boundaries staying in Gmail's default order.

use std::cmp::Ordering;

use anyhow::{Result, bail};
use chrono::{Duration, NaiveDate};

use crate::email::{
    address::Address,
    envelope::Envelope,
    flag::{Flag, IanaFlag},
    search::{
        filter::query::SearchEmailsFilterQuery,
        sort::query::{SearchEmailsSorter, SearchEmailsSorterKind, SearchEmailsSorterOrder},
    },
};

/// Translates the filter half of a shared query into a Gmail `q` value.
pub(super) fn filter_to_q(filter: &SearchEmailsFilterQuery) -> Result<String> {
    match filter {
        SearchEmailsFilterQuery::And(lhs, rhs) => {
            Ok(format!("{} {}", filter_to_q(lhs)?, filter_to_q(rhs)?))
        }
        SearchEmailsFilterQuery::Or(lhs, rhs) => {
            Ok(format!("{{{} {}}}", filter_to_q(lhs)?, filter_to_q(rhs)?))
        }
        SearchEmailsFilterQuery::Not(inner) => not_to_q(inner),
        SearchEmailsFilterQuery::Date(date) => Ok(format!(
            "after:{} before:{}",
            date_q(*date - Duration::days(1)),
            date_q(*date + Duration::days(1)),
        )),
        SearchEmailsFilterQuery::AfterDate(date) => Ok(format!("after:{}", date_q(*date))),
        SearchEmailsFilterQuery::From(pattern) => Ok(format!("from:{}", value_q(pattern))),
        SearchEmailsFilterQuery::To(pattern) => Ok(format!("to:{}", value_q(pattern))),
        SearchEmailsFilterQuery::Subject(pattern) => Ok(format!("subject:{}", value_q(pattern))),
        SearchEmailsFilterQuery::Body(pattern) => Ok(value_q(pattern)),
        SearchEmailsFilterQuery::Flag(flag) => flag_q(flag),
    }
}

/// Negates a filter, expanding through De Morgan so that `not (a and
/// b)` becomes an OR group of negations and `not (a or b)` an AND of
/// them, leaving every `-` on a leaf.
fn not_to_q(filter: &SearchEmailsFilterQuery) -> Result<String> {
    match filter {
        SearchEmailsFilterQuery::Not(inner) => filter_to_q(inner),
        SearchEmailsFilterQuery::And(lhs, rhs) => {
            Ok(format!("{{{} {}}}", not_to_q(lhs)?, not_to_q(rhs)?))
        }
        SearchEmailsFilterQuery::Or(lhs, rhs) => {
            Ok(format!("{} {}", not_to_q(lhs)?, not_to_q(rhs)?))
        }
        SearchEmailsFilterQuery::Date(date) => Ok(format!(
            "{{-after:{} -before:{}}}",
            date_q(*date - Duration::days(1)),
            date_q(*date + Duration::days(1)),
        )),
        SearchEmailsFilterQuery::AfterDate(date) => Ok(format!("-after:{}", date_q(*date))),
        SearchEmailsFilterQuery::From(pattern) => Ok(format!("-from:{}", value_q(pattern))),
        SearchEmailsFilterQuery::To(pattern) => Ok(format!("-to:{}", value_q(pattern))),
        SearchEmailsFilterQuery::Subject(pattern) => Ok(format!("-subject:{}", value_q(pattern))),
        SearchEmailsFilterQuery::Body(pattern) => Ok(format!("-{}", value_q(pattern))),
        SearchEmailsFilterQuery::Flag(flag) => Ok(format!("-{}", flag_q(flag)?)),
    }
}

/// Renders a pattern as a Gmail query value, quoting whitespace and
/// escaping embedded double quotes.
fn value_q(pattern: &str) -> String {
    if pattern.contains([' ', '\t', '"']) {
        format!("\"{}\"", pattern.replace('"', "\\\""))
    } else {
        pattern.to_string()
    }
}

/// Renders a date the way Gmail reads them, `yyyy/mm/dd`.
fn date_q(date: NaiveDate) -> String {
    date.format("%Y/%m/%d").to_string()
}

/// Maps a shared flag onto its Gmail operator, mirroring the
/// [`crate::gmail::backend`] label pairs. `\Answered` has none, Gmail
/// tracking replies in no searchable way.
fn flag_q(flag: &Flag) -> Result<String> {
    match flag.iana() {
        Some(IanaFlag::Seen) => Ok("is:read".into()),
        Some(IanaFlag::Flagged) => Ok("is:starred".into()),
        Some(IanaFlag::Draft) => Ok("is:draft".into()),
        Some(IanaFlag::Important) => Ok("is:important".into()),
        Some(IanaFlag::Junk) => Ok("in:spam".into()),
        Some(IanaFlag::Answered) => {
            bail!("Gmail search offers no operator for the answered flag")
        }
        _ => bail!(
            "Gmail search offers no operator for the {} flag",
            flag.raw()
        ),
    }
}

/// Orders a fetched page of envelopes by the sort half of a shared
/// query, keys applying left to right, the first sorter being the
/// primary key. Unset dates sort first ascending, address sorts look at
/// the first address, subjects compare case-insensitively.
pub(super) fn sort_envelopes(envelopes: &mut [Envelope], sorters: &[SearchEmailsSorter]) {
    envelopes.sort_by(|lhs, rhs| {
        for sorter in sorters {
            let ordering = match sorter.0 {
                SearchEmailsSorterKind::Date => lhs.date.cmp(&rhs.date),
                SearchEmailsSorterKind::From => compare_addresses(&lhs.from, &rhs.from),
                SearchEmailsSorterKind::To => compare_addresses(&lhs.to, &rhs.to),
                SearchEmailsSorterKind::Subject => {
                    lhs.subject.to_lowercase().cmp(&rhs.subject.to_lowercase())
                }
            };
            let ordering = match sorter.1 {
                SearchEmailsSorterOrder::Ascending => ordering,
                SearchEmailsSorterOrder::Descending => ordering.reverse(),
            };
            if ordering != Ordering::Equal {
                return ordering;
            }
        }
        Ordering::Equal
    });
}

/// Compares two address lists by their first address, the email being
/// the key, case-insensitively, a missing address sorting first.
fn compare_addresses(lhs: &[Address], rhs: &[Address]) -> Ordering {
    let key = |addrs: &[Address]| addrs.first().map(|addr| addr.email.to_lowercase());
    key(lhs).cmp(&key(rhs))
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use crate::email::search::query::SearchEmailsQuery;

    fn q(query: &str) -> String {
        let query = SearchEmailsQuery::from_str(query).unwrap();
        filter_to_q(&query.filter.unwrap()).unwrap()
    }

    fn q_err(query: &str) -> String {
        let query = SearchEmailsQuery::from_str(query).unwrap();
        filter_to_q(&query.filter.unwrap()).unwrap_err().to_string()
    }

    #[test]
    fn leaf_conditions() {
        assert_eq!(q("from alice"), "from:alice");
        assert_eq!(q("to bob@example.org"), "to:bob@example.org");
        assert_eq!(q("subject hello"), "subject:hello");
        assert_eq!(q("body world"), "world");
    }

    #[test]
    fn patterns_with_spaces_get_quoted() {
        assert_eq!(value_q("john doe"), "\"john doe\"");
        assert_eq!(value_q("said \"hi\""), "\"said \\\"hi\\\"\"");
        assert_eq!(value_q("alice"), "alice");
        assert_eq!(q("from alice"), "from:alice");
    }

    #[test]
    fn conjunctions_flatten_into_whitespace() {
        assert_eq!(q("from f and to t"), "from:f to:t");
        assert_eq!(q("from f and to t and subject s"), "from:f to:t subject:s",);
    }

    #[test]
    fn disjunctions_become_brace_groups() {
        assert_eq!(q("from f or to t"), "{from:f to:t}");
    }

    #[test]
    fn negated_leaves_get_a_dash() {
        assert_eq!(q("not from f"), "-from:f");
        assert_eq!(q("not flag seen"), "-is:read");
        assert_eq!(q("not flag flagged"), "-is:starred");
        assert_eq!(q("not flag draft"), "-is:draft");
        assert_eq!(q("not after 2026-09-08"), "-after:2026/09/08");
    }

    #[test]
    fn double_negation_cancels() {
        assert_eq!(q("not not from f"), "from:f");
        assert_eq!(q("not not flag seen"), "is:read");
    }

    #[test]
    fn negated_compounds_expand_through_de_morgan() {
        assert_eq!(q("not (from f or to t)"), "-from:f -to:t");
        assert_eq!(q("not (from f and to t)"), "{-from:f -to:t}");
        assert_eq!(q("not (not from f and not to t)"), "{from:f to:t}");
    }

    #[test]
    fn exact_dates_bracket_the_day() {
        assert_eq!(q("date 2026-09-08"), "after:2026/09/07 before:2026/09/09",);
        assert_eq!(
            q("not date 2026-09-08"),
            "{-after:2026/09/07 -before:2026/09/09}",
        );
    }

    #[test]
    fn flag_operators() {
        assert_eq!(q("flag seen"), "is:read");
        assert_eq!(q("flag flagged"), "is:starred");
        assert_eq!(q("flag draft"), "is:draft");
    }

    #[test]
    fn unsupported_flags_bail() {
        assert_eq!(
            q_err("flag answered"),
            "Gmail search offers no operator for the answered flag",
        );
    }

    #[test]
    fn the_unread_count_query_translates() {
        assert_eq!(q("not flag seen"), "-is:read");
    }
}
