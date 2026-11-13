//! # Gmail search
//!
//! Bridges the shared search query onto the Gmail `messages.list` `q`
//! syntax, the same one the Gmail search box speaks.
//!
//! The filter half becomes one `q` string: leaf conditions map to
//! operators (`from alice` becomes `from:alice`), conjunctions flatten
//! into whitespace, disjunctions into brace groups, and negations expand
//! through De Morgan so a `-` only ever prefixes a leaf, the shape Gmail
//! matches reliably. Two conditions have no Gmail counterpart and are
//! refused with `search-unsupported` rather than return wrong hits: `flag
//! answered`, replies being tracked in no searchable way, and flags outside
//! the set below.
//!
//! A date clause is sent as Unix seconds bounding the UTC day, Gmail
//! reading a bare date as midnight in PST. It compares them with the
//! message's internal date, when Gmail received it, where IMAP and the
//! local evaluator read the day written in `Date:`.
//!
//! The sort half has no server-side counterpart, `messages.list`
//! ordering being fixed newest-first, so the backend orders the fetched
//! page with the shared evaluator. The caveat: sorting a page is not
//! sorting the result set, page boundaries staying in Gmail's default
//! order.
//!
//! A pattern is a literal: what Gmail would read as an operator inside it
//! is neutralised (see [`value_q`]).

use anyhow::Result;
use chrono::{Duration, NaiveDate, NaiveTime};

use crate::email::{
    flag::{Flag, IanaFlag},
    search::{error::unsupported, filter::query::SearchEmailsFilterQuery},
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
            since_q(*date),
            day_start(next_day(*date)),
        )),
        SearchEmailsFilterQuery::AfterDate(date) => {
            Ok(format!("after:{}", since_q(next_day(*date))))
        }
        SearchEmailsFilterQuery::BeforeDate(date) => Ok(format!("before:{}", day_start(*date))),
        SearchEmailsFilterQuery::From(pattern) => Ok(format!("from:{}", value_q(filter, pattern)?)),
        SearchEmailsFilterQuery::To(pattern) => Ok(format!("to:{}", value_q(filter, pattern)?)),
        SearchEmailsFilterQuery::Cc(pattern) => Ok(format!("cc:{}", value_q(filter, pattern)?)),
        SearchEmailsFilterQuery::Bcc(pattern) => Ok(format!("bcc:{}", value_q(filter, pattern)?)),
        SearchEmailsFilterQuery::Address(pattern) => {
            let value = value_q(filter, pattern)?;
            Ok(format!(
                "{{from:{value} to:{value} cc:{value} bcc:{value}}}"
            ))
        }
        SearchEmailsFilterQuery::Subject(pattern) => {
            Ok(format!("subject:{}", value_q(filter, pattern)?))
        }
        SearchEmailsFilterQuery::Body(pattern) => value_q(filter, pattern),
        SearchEmailsFilterQuery::Flag(flag) => flag_q(filter, flag),
        SearchEmailsFilterQuery::Attachment => Ok(String::from("has:attachment")),
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
            since_q(*date),
            day_start(next_day(*date)),
        )),
        SearchEmailsFilterQuery::AfterDate(date) => {
            Ok(format!("-after:{}", since_q(next_day(*date))))
        }
        SearchEmailsFilterQuery::BeforeDate(date) => Ok(format!("-before:{}", day_start(*date))),
        SearchEmailsFilterQuery::From(pattern) => {
            Ok(format!("-from:{}", value_q(filter, pattern)?))
        }
        SearchEmailsFilterQuery::To(pattern) => Ok(format!("-to:{}", value_q(filter, pattern)?)),
        SearchEmailsFilterQuery::Cc(pattern) => Ok(format!("-cc:{}", value_q(filter, pattern)?)),
        SearchEmailsFilterQuery::Bcc(pattern) => Ok(format!("-bcc:{}", value_q(filter, pattern)?)),
        SearchEmailsFilterQuery::Address(pattern) => {
            let value = value_q(filter, pattern)?;
            Ok(format!(
                "-from:{value} -to:{value} -cc:{value} -bcc:{value}"
            ))
        }
        SearchEmailsFilterQuery::Subject(pattern) => {
            Ok(format!("-subject:{}", value_q(filter, pattern)?))
        }
        SearchEmailsFilterQuery::Body(pattern) => Ok(format!("-{}", value_q(filter, pattern)?)),
        SearchEmailsFilterQuery::Flag(flag) => Ok(format!("-{}", flag_q(filter, flag)?)),
        SearchEmailsFilterQuery::Attachment => Ok(String::from("-has:attachment")),
    }
}

/// Renders the pattern of `clause` as a Gmail query value matching it
/// literally, refusing a pattern left with no word.
///
/// Gmail reads `OR`, `AND`, braces and parentheses even inside quotes and
/// has no escape, so quotes, braces and parentheses become spaces (Gmail
/// matches words, skipping punctuation) and a standalone `OR` or `AND` is
/// lowercased (its operators are case-sensitive). Anything but a bare word
/// is then quoted, so no `-` or `:` reads as an operator.
fn value_q(clause: &SearchEmailsFilterQuery, pattern: &str) -> Result<String> {
    let words: Vec<&str> = pattern
        .split(|c: char| c.is_whitespace() || matches!(c, '"' | '{' | '}' | '(' | ')'))
        .filter(|word| !word.is_empty())
        .collect();

    if words.is_empty() {
        return Err(unsupported(
            clause,
            "Gmail search matches words, and it holds none",
        ));
    }

    let words: Vec<String> = words
        .into_iter()
        .map(|word| match word {
            "OR" | "AND" => word.to_lowercase(),
            word => word.to_string(),
        })
        .collect();

    let bare = |c: char| c.is_alphanumeric() || matches!(c, '@' | '.' | '_' | '+');
    match words.as_slice() {
        [word] if word.chars().all(bare) => Ok(word.clone()),
        words => Ok(format!("\"{}\"", words.join(" "))),
    }
}

/// The first second of `day` in UTC, as Unix seconds.
///
/// Gmail reads a bare `yyyy/mm/dd` as midnight in the PST timezone, and
/// takes Unix seconds as an exact instant instead
/// (<https://developers.google.com/workspace/gmail/api/guides/filtering>),
/// so every day boundary is sent in seconds, UTC.
fn day_start(day: NaiveDate) -> i64 {
    day.and_time(NaiveTime::MIN).and_utc().timestamp()
}

/// The `after:` value keeping the mail from the start of `day` on.
///
/// One second before the boundary, so a message dated on it is kept
/// whether Gmail compares `after:` strictly or not.
fn since_q(day: NaiveDate) -> i64 {
    day_start(day) - 1
}

/// The day after `day`.
fn next_day(day: NaiveDate) -> NaiveDate {
    day + Duration::days(1)
}

/// Maps a shared flag onto its Gmail operator, mirroring the
/// [`crate::gmail::backend`] label pairs, `clause` being refused whole when
/// there is none: `\Answered`, Gmail tracking replies in no searchable way,
/// and the keywords.
fn flag_q(clause: &SearchEmailsFilterQuery, flag: &Flag) -> Result<String> {
    match flag.iana() {
        Some(IanaFlag::Seen) => Ok("is:read".into()),
        Some(IanaFlag::Flagged) => Ok("is:starred".into()),
        Some(IanaFlag::Draft) => Ok("is:draft".into()),
        Some(IanaFlag::Important) => Ok("is:important".into()),
        Some(IanaFlag::Junk) => Ok("in:spam".into()),
        _ => Err(unsupported(
            clause,
            "Gmail search offers no operator for this flag",
        )),
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use pimalaya_cli::error::code_of;

    use super::*;
    use crate::{email::search::query::SearchEmailsQuery, error::ErrorCode};

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
        assert_eq!(q("from \"john doe\""), "from:\"john doe\"");
        assert_eq!(q("from alice@example.org"), "from:alice@example.org");
    }

    #[test]
    fn patterns_cannot_widen_the_search() {
        assert_eq!(
            q("subject \"rapport OR kangaroo\""),
            "subject:\"rapport or kangaroo\""
        );
        assert_eq!(
            q("subject \"rapport} OR {kangaroo\""),
            "subject:\"rapport or kangaroo\""
        );
        assert_eq!(q("from \"alice) AND (dave\""), "from:\"alice and dave\"");
        assert_eq!(
            q(r#"body "zebrafish\" OR kangaroo""#),
            "\"zebrafish or kangaroo\""
        );
    }

    #[test]
    fn operator_lookalikes_get_quoted() {
        assert_eq!(q("body \"from:alice\""), "\"from:alice\"");
        assert_eq!(q("not body \"-draft\""), "-\"-draft\"");
    }

    #[test]
    fn patterns_without_a_word_bail() {
        assert!(q_err("subject \"()\"").contains("holds none"));
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
        assert_eq!(q("not after 2026-09-08"), "-after:1788911999");
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
    fn exact_dates_bracket_the_utc_day() {
        assert_eq!(q("date 2026-09-08"), "after:1788825599 before:1788912000");
        assert_eq!(
            q("not date 2026-09-08"),
            "{-after:1788825599 -before:1788912000}",
        );
    }

    #[test]
    fn after_starts_on_the_next_utc_day() {
        // NOTE: `after` is strict, and Gmail's bare `after:2026/09/08`
        // kept that day's mail too, from its midnight in PST.
        assert_eq!(q("after 2026-09-08"), "after:1788911999");
    }

    #[test]
    fn flag_operators() {
        assert_eq!(q("flag seen"), "is:read");
        assert_eq!(q("flag flagged"), "is:starred");
        assert_eq!(q("flag draft"), "is:draft");
    }

    #[test]
    fn unsupported_flags_bail() {
        let query = SearchEmailsQuery::from_str("flag answered").unwrap();
        let err = filter_to_q(&query.filter.unwrap()).unwrap_err();

        assert_eq!(
            err.to_string(),
            "Cannot search `flag answered`: Gmail search offers no operator for this flag",
        );
        assert_eq!(
            code_of::<ErrorCode>(&err),
            Some(ErrorCode::SearchUnsupported),
        );
        assert!(q_err("flag $label").contains("flag $label"));
    }

    #[test]
    fn copies_addresses_dates_and_attachments() {
        assert_eq!(q("cc carol"), "cc:carol");
        assert_eq!(q("bcc dave"), "bcc:dave");
        assert_eq!(q("address eve"), "{from:eve to:eve cc:eve bcc:eve}");
        assert_eq!(q("before 2026-09-08"), "before:1788825600");
        assert_eq!(q("attachment"), "has:attachment");
        assert_eq!(q("not address eve"), "-from:eve -to:eve -cc:eve -bcc:eve");
        assert_eq!(q("not attachment"), "-has:attachment");
        assert_eq!(q("not before 2026-09-08"), "-before:1788825600");
    }

    #[test]
    fn the_unread_count_query_translates() {
        assert_eq!(q("not flag seen"), "-is:read");
    }
}
