//! # Gmail search
//!
//! Bridges the unified search query onto the Gmail `messages.list` `q`
//! syntax, the same one the Gmail search box speaks.
//!
//! The filter half becomes one `q` string, negations expanded through De
//! Morgan so a `-` only prefixes a leaf, the shape Gmail matches reliably.
//! `flag answered` (replies are not searchable) and the keywords are refused
//! with `search-unsupported` rather than return wrong hits.
//!
//! A date clause is sent as Unix seconds bounding the UTC day, Gmail
//! reading a bare date as midnight in PST. It compares them with the
//! message's internal date, when Gmail received it, where IMAP and the
//! local evaluator read the day written in `Date:`.
//!
//! The sort half has no server-side counterpart, `messages.list` being
//! fixed newest-first, so the backend sorts the fetched page: page
//! boundaries stay in Gmail's default order.
//!
//! A pattern is a literal: what Gmail would read as an operator inside it
//! is neutralised (see `value_q`).

use anyhow::Result;
use chrono::{Duration, NaiveDate, NaiveTime};

use crate::unified::email::{
    flag::{Flag, IanaFlag},
    search::{error::unsupported, filter::query::SearchEmailsFilterQuery},
};

/// Translates the filter half of a unified query into a Gmail `q` value.
pub(super) fn filter_to_q(filter: &SearchEmailsFilterQuery) -> Result<String> {
    match filter {
        SearchEmailsFilterQuery::And(lhs, rhs) => {
            Ok(format!("{} {}", filter_to_q(lhs)?, filter_to_q(rhs)?))
        }
        SearchEmailsFilterQuery::Or(lhs, rhs) => Ok(any_of(&filter_to_q(lhs)?, &filter_to_q(rhs)?)),
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
        SearchEmailsFilterQuery::AfterInstant(instant) => {
            Ok(format!("after:{}", instant.timestamp()))
        }
        SearchEmailsFilterQuery::BeforeInstant(instant) => {
            Ok(format!("before:{}", instant.timestamp()))
        }
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
        SearchEmailsFilterQuery::MessageId(id) => {
            Ok(format!("rfc822msgid:{}", value_q(filter, id)?))
        }
    }
}

/// Negates a filter through De Morgan, leaving every `-` on a leaf.
fn not_to_q(filter: &SearchEmailsFilterQuery) -> Result<String> {
    match filter {
        SearchEmailsFilterQuery::Not(inner) => filter_to_q(inner),
        SearchEmailsFilterQuery::And(lhs, rhs) => Ok(any_of(&not_to_q(lhs)?, &not_to_q(rhs)?)),
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
        SearchEmailsFilterQuery::AfterInstant(instant) => {
            Ok(format!("-after:{}", instant.timestamp()))
        }
        SearchEmailsFilterQuery::BeforeInstant(instant) => {
            Ok(format!("-before:{}", instant.timestamp()))
        }
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
        SearchEmailsFilterQuery::MessageId(id) => {
            Ok(format!("-rfc822msgid:{}", value_q(filter, id)?))
        }
    }
}

/// The brace group matching `lhs` or `rhs`, a multi-term side in parentheses
/// since Gmail reads each term inside braces as an alternative.
fn any_of(lhs: &str, rhs: &str) -> String {
    format!("{{{} {}}}", group(lhs), group(rhs))
}

/// `q` in parentheses when it is a conjunction, a blank outside any
/// quote, brace or parenthesis separating two of its terms.
fn group(q: &str) -> String {
    let mut depth = 0usize;
    let mut quoted = false;

    let conjunction = q.chars().any(|c| {
        match c {
            '"' => quoted = !quoted,
            '{' | '(' if !quoted => depth += 1,
            '}' | ')' if !quoted => depth = depth.saturating_sub(1),
            c if c.is_whitespace() => return !quoted && depth == 0,
            _ => (),
        }
        false
    });

    if conjunction {
        format!("({q})")
    } else {
        q.to_string()
    }
}

/// Renders the pattern of `clause` as a Gmail query value matching it
/// literally, refusing a pattern left with no word.
///
/// Gmail reads `OR`, `AND`, braces and parentheses even quoted, with no
/// escape: those marks become spaces (it matches words), a bare `OR`/`AND`
/// is lowercased, and the rest is quoted so no `-` or `:` is an operator.
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
/// Gmail reads a bare `yyyy/mm/dd` as midnight PST but seconds as an exact
/// instant
/// (<https://developers.google.com/workspace/gmail/api/guides/filtering>).
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

/// Maps a unified flag onto its Gmail operator, as the
/// [`crate::gmail::backend`] label pairs, refusing `\Answered` and keywords.
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
    use crate::{error::ErrorCode, unified::email::search::query::SearchEmailsQuery};

    fn q(query: &str) -> String {
        let query = SearchEmailsQuery::from_str(query).unwrap();
        filter_to_q(&query.filter.unwrap()).unwrap()
    }

    fn q_err(query: &str) -> String {
        let query = SearchEmailsQuery::from_str(query).unwrap();
        filter_to_q(&query.filter.unwrap()).unwrap_err().to_string()
    }

    /// An instant goes out as its Unix seconds, `after:` and `before:`
    /// being strict.
    #[test]
    fn an_instant_is_sent_in_seconds() {
        assert_eq!(q("after 2026-11-14T10:00:00+01:00"), "after:1794646800");
        assert_eq!(q("not before 2026-11-14T09:00:00Z"), "-before:1794646800");
    }

    #[test]
    fn leaf_conditions() {
        assert_eq!(q("from alice"), "from:alice");
        assert_eq!(q("to bob@example.org"), "to:bob@example.org");
        assert_eq!(q("subject hello"), "subject:hello");
        assert_eq!(q("body world"), "world");
    }

    #[test]
    fn message_id_conditions() {
        assert_eq!(
            q("message-id id123@example.org"),
            "rfc822msgid:id123@example.org"
        );
        assert_eq!(
            q("message-id <id123@example.org>"),
            "rfc822msgid:id123@example.org"
        );
        assert_eq!(
            q("not message-id id123@example.org"),
            "-rfc822msgid:id123@example.org"
        );
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

    /// Gmail reads each term inside braces as an alternative, so a
    /// conjunction under a disjunction keeps its parentheses.
    #[test]
    fn conjunctions_under_disjunctions_keep_their_group() {
        assert_eq!(
            q("from alice and subject invoice or from bob"),
            "{(from:alice subject:invoice) from:bob}",
        );
        assert_eq!(
            q("date 2026-09-08 or from alice"),
            "{(after:1788825599 before:1788912000) from:alice}",
        );
        assert_eq!(
            q("from a or (to b and (cc c or subject \"d e\"))"),
            "{from:a (to:b {cc:c subject:\"d e\"})}",
        );
        assert_eq!(
            q("not ((from a or to b) and subject c)"),
            "{(-from:a -to:b) -subject:c}",
        );
        assert_eq!(
            q("not (address e and to b)"),
            "{(-from:e -to:e -cc:e -bcc:e) -to:b}",
        );
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
