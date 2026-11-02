//! # Search evaluation
//!
//! Client-side evaluation of a shared search query against an
//! [`Envelope`], for the backends that search locally.
//!
//! The filter is matched against each envelope, `body` clauses reaching
//! for the raw bytes, then the sort chain orders the hits, `Date:`
//! descending by default. Date clauses read the `Date:` header, as the
//! server-side backends do. The copy recipients, which an envelope does not
//! carry, come from [`EnvelopeCopies`] when the caller has them, else from
//! the raw headers, read only when a clause needs them.

use std::cmp::Ordering;

use chrono::{DateTime, FixedOffset, NaiveDate};
use mime::{
    header::marker::{BCC, CC},
    tree::{cst::MimeCst, parse::MimeParseOptions},
};

use crate::email::{
    address::{self, Address},
    envelope::Envelope,
    search::{
        filter::query::SearchEmailsFilterQuery,
        sort::query::{SearchEmailsSorter, SearchEmailsSorterKind, SearchEmailsSorterOrder},
    },
};

/// The copy recipients of a message, which an [`Envelope`] does not carry.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EnvelopeCopies {
    /// The `Cc:` recipients.
    pub cc: Vec<Address>,
    /// The `Bcc:` recipients.
    pub bcc: Vec<Address>,
}

impl EnvelopeCopies {
    /// Reads the copy recipients off a raw message's headers, none when it
    /// does not parse.
    pub fn from_raw(raw: &[u8]) -> Self {
        let opts = MimeParseOptions {
            structure_depth: Some(0),
            ..Default::default()
        };
        let message = MimeCst::parse_with(raw, opts);
        let root = &message.root;

        Self {
            cc: root
                .header::<CC>()
                .map(|list| address::from_mime(&list))
                .unwrap_or_default(),
            bcc: root
                .header::<BCC>()
                .map(|list| address::from_mime(&list))
                .unwrap_or_default(),
        }
    }
}

/// Evaluates a filter against an envelope, a `body` clause re-parsing the
/// raw message.
///
/// The copy recipients are `copies` when the caller has them, else read
/// off `raw` once, when the filter holds a clause over them.
pub fn matches_filter(
    envelope: &Envelope,
    copies: Option<&EnvelopeCopies>,
    raw: &[u8],
    filter: &SearchEmailsFilterQuery,
) -> bool {
    let read;
    let copies = match copies {
        Some(copies) => copies,
        None if reads_copies(filter) => {
            read = EnvelopeCopies::from_raw(raw);
            &read
        }
        None => &EnvelopeCopies::default(),
    };

    matches(envelope, copies, raw, filter)
}

/// Sorts envelopes by the given chain, or by `Date:` descending when the
/// query carries no sort clause.
pub fn sort_envelopes(envelopes: &mut [Envelope], sort: Option<&[SearchEmailsSorter]>) {
    match sort {
        Some(chain) if !chain.is_empty() => envelopes.sort_by(|a, b| compare_with(a, b, chain)),
        _ => envelopes.sort_by_key(|envelope| std::cmp::Reverse(envelope.date)),
    }
}

fn matches(
    envelope: &Envelope,
    copies: &EnvelopeCopies,
    raw: &[u8],
    filter: &SearchEmailsFilterQuery,
) -> bool {
    use SearchEmailsFilterQuery as Q;

    match filter {
        Q::And(left, right) => {
            matches(envelope, copies, raw, left) && matches(envelope, copies, raw, right)
        }
        Q::Or(left, right) => {
            matches(envelope, copies, raw, left) || matches(envelope, copies, raw, right)
        }
        Q::Not(inner) => !matches(envelope, copies, raw, inner),
        Q::Date(target) => same_day(envelope.date, *target),
        Q::AfterDate(target) => after_day(envelope.date, *target),
        Q::BeforeDate(target) => before_day(envelope.date, *target),
        Q::From(pattern) => addresses_contain(&envelope.from, pattern),
        Q::To(pattern) => addresses_contain(&envelope.to, pattern),
        Q::Cc(pattern) => addresses_contain(&copies.cc, pattern),
        Q::Bcc(pattern) => addresses_contain(&copies.bcc, pattern),
        Q::Address(pattern) => [&envelope.from, &envelope.to, &copies.cc, &copies.bcc]
            .into_iter()
            .any(|addrs| addresses_contain(addrs, pattern)),
        Q::Subject(pattern) => contains_ci(&envelope.subject, pattern),
        Q::Body(pattern) => body_contains(raw, pattern),
        Q::Flag(flag) => envelope.flags.contains(flag),
        Q::Attachment => has_attachment(envelope, raw),
    }
}

/// Whether a filter holds a clause over the copy recipients anywhere.
fn reads_copies(filter: &SearchEmailsFilterQuery) -> bool {
    use SearchEmailsFilterQuery as Q;

    match filter {
        Q::And(left, right) | Q::Or(left, right) => reads_copies(left) || reads_copies(right),
        Q::Not(inner) => reads_copies(inner),
        Q::Cc(_) | Q::Bcc(_) | Q::Address(_) => true,
        _ => false,
    }
}

/// Whether the message holds an attachment: the envelope's mark when the
/// backend set it, else the raw message's parts, none when it is empty.
fn has_attachment(envelope: &Envelope, raw: &[u8]) -> bool {
    if let Some(has_attachment) = envelope.has_attachment {
        return has_attachment;
    }

    !raw.is_empty() && MimeCst::parse(raw).root.body_lists().has_attachment()
}

fn body_contains(raw: &[u8], pattern: &str) -> bool {
    let message = MimeCst::parse(raw);
    let lists = message.root.body_lists();
    let needle = pattern.as_bytes();

    lists.text.iter().chain(&lists.html).any(|part| {
        part.text()
            .is_some_and(|text| contains_ignore_ascii_case(text.as_bytes(), needle))
    })
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

fn before_day(date: Option<DateTime<FixedOffset>>, target: NaiveDate) -> bool {
    date.map(|d| d.date_naive() < target).unwrap_or(false)
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
            None,
            &raw,
            &SearchEmailsFilterQuery::From("ALICE".into())
        ));
        assert!(!matches_filter(
            &env,
            None,
            &raw,
            &SearchEmailsFilterQuery::From("bob".into())
        ));
        assert!(matches_filter(
            &env,
            None,
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
            None,
            &raw,
            &SearchEmailsFilterQuery::Body("QUARTERLY".into())
        ));
        assert!(!matches_filter(
            &env,
            None,
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
            None,
            &raw,
            &SearchEmailsFilterQuery::Date(naive(2026, 5, 15))
        ));
        assert!(!matches_filter(
            &env,
            None,
            &raw,
            &SearchEmailsFilterQuery::Date(naive(2026, 5, 14))
        ));
        assert!(matches_filter(
            &env,
            None,
            &raw,
            &SearchEmailsFilterQuery::AfterDate(naive(2026, 5, 14))
        ));
        assert!(!matches_filter(
            &env,
            None,
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
            None,
            &raw,
            &SearchEmailsFilterQuery::Flag(Flag::from_iana(IanaFlag::Seen))
        ));
        assert!(!matches_filter(
            &env,
            None,
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

    #[test]
    fn copies_are_read_off_the_raw_headers_when_none_are_given() {
        let env = envelope();
        let raw =
            b"From: a@b\r\nCc: Carol <carol@example.org>\r\nBcc: dave@example.org\r\n\r\nbody";

        assert!(matches_filter(
            &env,
            None,
            raw,
            &SearchEmailsFilterQuery::Cc("carol".into())
        ));
        assert!(matches_filter(
            &env,
            None,
            raw,
            &SearchEmailsFilterQuery::Bcc("DAVE".into())
        ));
        assert!(!matches_filter(
            &env,
            None,
            raw,
            &SearchEmailsFilterQuery::Cc("dave".into())
        ));
    }

    #[test]
    fn given_copies_win_over_the_raw_headers() {
        let env = envelope();
        let copies = EnvelopeCopies {
            cc: vec![Address {
                name: None,
                email: String::from("zoe@example.org"),
            }],
            bcc: Vec::new(),
        };
        let raw = b"Cc: carol@example.org\r\n\r\n";

        assert!(matches_filter(
            &env,
            Some(&copies),
            raw,
            &SearchEmailsFilterQuery::Cc("zoe".into())
        ));
        assert!(!matches_filter(
            &env,
            Some(&copies),
            raw,
            &SearchEmailsFilterQuery::Cc("carol".into())
        ));
    }

    #[test]
    fn address_reads_every_role() {
        let env = envelope();
        let raw = b"Cc: carol@example.org\r\n\r\n";

        for pattern in ["alice", "team", "carol"] {
            assert!(matches_filter(
                &env,
                None,
                raw,
                &SearchEmailsFilterQuery::Address(pattern.into())
            ));
        }
        assert!(!matches_filter(
            &env,
            None,
            raw,
            &SearchEmailsFilterQuery::Address("nobody".into())
        ));
    }

    #[test]
    fn before_is_strict() {
        let env = envelope();

        assert!(matches_filter(
            &env,
            None,
            &[],
            &SearchEmailsFilterQuery::BeforeDate(naive(2026, 5, 16))
        ));
        assert!(!matches_filter(
            &env,
            None,
            &[],
            &SearchEmailsFilterQuery::BeforeDate(naive(2026, 5, 15))
        ));
    }

    #[test]
    fn attachment_reads_the_mark_else_the_parts() {
        let mut env = envelope();
        let raw = b"Content-Type: multipart/mixed; boundary=m\r\n\r\n--m\r\nContent-Type: text/plain\r\n\r\nhi\r\n--m\r\nContent-Type: application/pdf\r\nContent-Disposition: attachment; filename=a.pdf\r\n\r\nPDF\r\n--m--\r\n";

        assert!(matches_filter(
            &env,
            None,
            raw,
            &SearchEmailsFilterQuery::Attachment
        ));
        assert!(!matches_filter(
            &env,
            None,
            &[],
            &SearchEmailsFilterQuery::Attachment
        ));

        env.has_attachment = Some(false);
        assert!(!matches_filter(
            &env,
            None,
            raw,
            &SearchEmailsFilterQuery::Attachment
        ));
    }
}
