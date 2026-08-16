//! Client-side evaluation of the shared search query against an
//! [`Envelope`].
//!
//! Used by the backends that search locally (Maildir, m2dir): the
//! filter is matched against each envelope (with the raw bytes for
//! `body` clauses), then the sort chain (or the default `Date:`
//! descending) orders the hits. The date clauses target the `Date:`
//! header (sent-at), consistent with the server-side backends.

use std::cmp::Ordering;

use chrono::{DateTime, FixedOffset, NaiveDate};
use mail_parser::MessageParser;

use crate::email::{
    address::Address,
    envelope::Envelope,
    search::{
        filter::query::SearchEmailsFilterQuery,
        sort::query::{SearchEmailsSorter, SearchEmailsSorterKind, SearchEmailsSorterOrder},
    },
};

/// Evaluates `filter` against `envelope`; `body` clauses re-parse `raw`.
pub fn matches_filter(envelope: &Envelope, raw: &[u8], filter: &SearchEmailsFilterQuery) -> bool {
    use SearchEmailsFilterQuery as Q;

    match filter {
        Q::And(left, right) => {
            matches_filter(envelope, raw, left) && matches_filter(envelope, raw, right)
        }
        Q::Or(left, right) => {
            matches_filter(envelope, raw, left) || matches_filter(envelope, raw, right)
        }
        Q::Not(inner) => !matches_filter(envelope, raw, inner),
        Q::Date(target) => same_day(envelope.date, *target),
        Q::AfterDate(target) => after_day(envelope.date, *target),
        Q::From(pattern) => addresses_contain(&envelope.from, pattern),
        Q::To(pattern) => addresses_contain(&envelope.to, pattern),
        Q::Subject(pattern) => contains_ci(&envelope.subject, pattern),
        Q::Body(pattern) => body_contains(raw, pattern),
        Q::Flag(flag) => envelope.flags.contains(flag),
    }
}

/// Sorts `envelopes` by the given chain, or by `Date:` descending when
/// no sort clause is present.
pub fn sort_envelopes(envelopes: &mut [Envelope], sort: Option<&[SearchEmailsSorter]>) {
    match sort {
        Some(chain) if !chain.is_empty() => envelopes.sort_by(|a, b| compare_with(a, b, chain)),
        _ => envelopes.sort_by_key(|envelope| std::cmp::Reverse(envelope.date)),
    }
}

fn body_contains(raw: &[u8], pattern: &str) -> bool {
    let Some(msg) = MessageParser::new().parse(raw) else {
        return false;
    };
    let needle = pattern.as_bytes();
    for part in msg.text_bodies() {
        if contains_ignore_ascii_case(part.contents(), needle) {
            return true;
        }
    }
    for part in msg.html_bodies() {
        if contains_ignore_ascii_case(part.contents(), needle) {
            return true;
        }
    }
    false
}

/// Case-insensitive substring search on raw bytes, ASCII semantics.
fn contains_ignore_ascii_case(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    if needle.len() > haystack.len() {
        return false;
    }
    haystack
        .windows(needle.len())
        .any(|w| w.eq_ignore_ascii_case(needle))
}

fn same_day(date: Option<DateTime<FixedOffset>>, target: NaiveDate) -> bool {
    date.map(|d| d.date_naive() == target).unwrap_or(false)
}

fn after_day(date: Option<DateTime<FixedOffset>>, target: NaiveDate) -> bool {
    date.map(|d| d.date_naive() > target).unwrap_or(false)
}

fn addresses_contain(addrs: &[Address], pattern: &str) -> bool {
    let needle = pattern.to_lowercase();
    addrs.iter().any(|addr| {
        let email_hit = addr.email.to_lowercase().contains(&needle);
        let name_hit = addr
            .name
            .as_deref()
            .map(|n| n.to_lowercase().contains(&needle))
            .unwrap_or(false);
        email_hit || name_hit
    })
}

fn contains_ci(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(&needle.to_lowercase())
}

fn compare_with(left: &Envelope, right: &Envelope, sort: &[SearchEmailsSorter]) -> Ordering {
    for SearchEmailsSorter(kind, order) in sort {
        let cmp = match kind {
            SearchEmailsSorterKind::Date => left.date.cmp(&right.date),
            SearchEmailsSorterKind::From => {
                first_addr_key(&left.from).cmp(&first_addr_key(&right.from))
            }
            SearchEmailsSorterKind::To => first_addr_key(&left.to).cmp(&first_addr_key(&right.to)),
            SearchEmailsSorterKind::Subject => left.subject.cmp(&right.subject),
        };
        let cmp = match order {
            SearchEmailsSorterOrder::Ascending => cmp,
            SearchEmailsSorterOrder::Descending => cmp.reverse(),
        };
        if cmp != Ordering::Equal {
            return cmp;
        }
    }
    Ordering::Equal
}

fn first_addr_key(addrs: &[Address]) -> Option<String> {
    addrs.first().map(|a| {
        a.name
            .as_deref()
            .map(str::to_lowercase)
            .unwrap_or_else(|| a.email.to_lowercase())
    })
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;

    use super::*;
    use crate::email::flag::{Flag, IanaFlag};

    fn envelope() -> Envelope {
        Envelope {
            id: String::from("1"),
            message_id: None,
            in_reply_to: Vec::new(),
            flags: Default::default(),
            subject: String::from("Release notes"),
            from: vec![Address {
                name: Some(String::from("Alice")),
                email: String::from("alice@example.org"),
            }],
            to: vec![Address {
                name: None,
                email: String::from("team@example.org"),
            }],
            date: DateTime::parse_from_rfc3339("2026-05-15T10:00:00+00:00").ok(),
            size: 1024,
            has_attachment: None,
        }
    }

    fn naive(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn raw_with_body(body: &str) -> Vec<u8> {
        format!("Subject: x\r\nFrom: a@b\r\nDate: Thu, 15 May 2026 10:00:00 +0000\r\n\r\n{body}")
            .into_bytes()
    }

    #[test]
    fn from_and_subject_match_case_insensitively() {
        let env = envelope();
        let raw = Vec::new();
        assert!(matches_filter(
            &env,
            &raw,
            &SearchEmailsFilterQuery::From("ALICE".into())
        ));
        assert!(!matches_filter(
            &env,
            &raw,
            &SearchEmailsFilterQuery::From("bob".into())
        ));
        assert!(matches_filter(
            &env,
            &raw,
            &SearchEmailsFilterQuery::Subject("release".into())
        ));
    }

    #[test]
    fn body_filter_scans_message_bytes() {
        let env = envelope();
        let raw = raw_with_body("Hello, this is a quarterly review.");
        assert!(matches_filter(
            &env,
            &raw,
            &SearchEmailsFilterQuery::Body("QUARTERLY".into())
        ));
        assert!(!matches_filter(
            &env,
            &raw,
            &SearchEmailsFilterQuery::Body("absent".into())
        ));
    }

    #[test]
    fn date_clauses_target_sent_at_header() {
        let env = envelope();
        let raw = Vec::new();
        assert!(matches_filter(
            &env,
            &raw,
            &SearchEmailsFilterQuery::Date(naive(2026, 5, 15))
        ));
        assert!(!matches_filter(
            &env,
            &raw,
            &SearchEmailsFilterQuery::Date(naive(2026, 5, 14))
        ));
        assert!(matches_filter(
            &env,
            &raw,
            &SearchEmailsFilterQuery::AfterDate(naive(2026, 5, 14))
        ));
        assert!(!matches_filter(
            &env,
            &raw,
            &SearchEmailsFilterQuery::AfterDate(naive(2026, 5, 15))
        ));
    }

    #[test]
    fn flag_match_uses_envelope_flags() {
        let mut env = envelope();
        let raw = Vec::new();
        env.flags.insert(Flag::from_iana(IanaFlag::Seen));
        assert!(matches_filter(
            &env,
            &raw,
            &SearchEmailsFilterQuery::Flag(Flag::from_iana(IanaFlag::Seen))
        ));
        assert!(!matches_filter(
            &env,
            &raw,
            &SearchEmailsFilterQuery::Flag(Flag::from_iana(IanaFlag::Flagged))
        ));
    }

    #[test]
    fn sort_by_date_descending_returns_newer_first() {
        let mut older = envelope();
        older.date = DateTime::parse_from_rfc3339("2026-01-01T00:00:00+00:00").ok();
        let newer = envelope();
        let sort = vec![SearchEmailsSorter(
            SearchEmailsSorterKind::Date,
            SearchEmailsSorterOrder::Descending,
        )];
        assert_eq!(compare_with(&newer, &older, &sort), Ordering::Less);
    }
}
