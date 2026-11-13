//! # Search sort evaluation
//!
//! Client-side ordering of envelopes by the sort half of a shared search
//! query, for the backends that search locally and for Graph, which sorts
//! after searching.
//!
//! The keys mean what they mean to an IMAP server (RFC 5256 section 3):
//! the base subject, `Re:` and `Fwd:` stripped, and the mailbox of the
//! first address, both compared under `i;ascii-casemap`.

use std::cmp::{Ordering, Reverse};

use crate::email::{
    address::Address,
    envelope::Envelope,
    search::sort::query::{SearchEmailsSorter, SearchEmailsSorterKind, SearchEmailsSorterOrder},
};

/// Sorts envelopes by the given chain, or by `Date:` descending when the
/// query carries no sort clause.
///
/// The sort is stable: envelopes equal on every key keep their order.
pub fn sort_envelopes(envelopes: &mut [Envelope], sort: Option<&[SearchEmailsSorter]>) {
    match sort {
        Some(chain) if !chain.is_empty() => envelopes.sort_by(|a, b| compare_with(a, b, chain)),
        _ => envelopes.sort_by_key(|envelope| Reverse(envelope.date)),
    }
}

fn compare_with(left: &Envelope, right: &Envelope, sort: &[SearchEmailsSorter]) -> Ordering {
    for SearchEmailsSorter(kind, order) in sort {
        let cmp = match kind {
            SearchEmailsSorterKind::Date => left.date.cmp(&right.date),
            SearchEmailsSorterKind::From => casemap_cmp(mailbox(&left.from), mailbox(&right.from)),
            SearchEmailsSorterKind::To => casemap_cmp(mailbox(&left.to), mailbox(&right.to)),
            SearchEmailsSorterKind::Subject => {
                casemap_cmp(&base_subject(&left.subject), &base_subject(&right.subject))
            }
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

/// Compares two strings under `i;ascii-casemap` (RFC 4790): ASCII letters
/// folded to upper case, every other octet compared as is.
fn casemap_cmp(left: &str, right: &str) -> Ordering {
    let fold = |s: &str| {
        s.bytes()
            .map(|b| b.to_ascii_uppercase())
            .collect::<Vec<_>>()
    };
    fold(left).cmp(&fold(right))
}

/// The mailbox, the local part, of the first address, empty when there
/// is none (RFC 5256 section 3, `FROM` and `TO`).
fn mailbox(addrs: &[Address]) -> &str {
    addrs.first().map_or("", |addr| {
        addr.email
            .rsplit_once('@')
            .map_or(addr.email.as_str(), |(local, _)| local)
    })
}

/// Extracts the base subject of RFC 5256 section 2.1 from a decoded
/// subject: blanks collapsed, `(fwd)` trailers, `Re:`, `Fw:` and `Fwd:`
/// leaders, `[blob]` prefixes and `[fwd: ...]` wrappers stripped.
pub fn base_subject(subject: &str) -> String {
    // Step 1: every run of blanks becomes one space.
    let mut base = subject.split_whitespace().collect::<Vec<_>>().join(" ");

    loop {
        // Step 2: trailing `(fwd)` and blanks.
        loop {
            let trimmed = base.trim_end();
            let trimmed = match trimmed.len().checked_sub(5) {
                Some(at)
                    if trimmed.is_char_boundary(at)
                        && trimmed[at..].eq_ignore_ascii_case("(fwd)") =>
                {
                    &trimmed[..at]
                }
                _ => trimmed,
            };
            if trimmed.len() == base.len() {
                break;
            }
            base.truncate(trimmed.len());
        }

        // Steps 3 to 5: leaders, then a blob leaving a non-empty base.
        loop {
            let mut rest = base.as_str();
            while let Some(after) = strip_leader(rest) {
                rest = after;
            }
            if let Some(after) = strip_blob(rest)
                && !after.is_empty()
            {
                rest = after;
            }
            if rest.len() == base.len() {
                break;
            }
            base = rest.to_owned();
        }

        // Step 6: a `[fwd: ...]` wrapper, then again from step 2.
        let unwrapped = base
            .get(..5)
            .filter(|head| head.eq_ignore_ascii_case("[fwd:"))
            .and_then(|_| base.strip_suffix(']'))
            .map(|inner| inner[5..].to_owned());
        match unwrapped {
            Some(inner) => base = inner,
            None => return base,
        }
    }
}

/// Strips one `subj-leader`: a leading blank, or blobs followed by `re`,
/// `fw` or `fwd`, blanks, an optional blob and a colon.
fn strip_leader(s: &str) -> Option<&str> {
    if let Some(rest) = s.strip_prefix(' ') {
        return Some(rest);
    }

    let mut rest = s;
    while let Some(after) = strip_blob(rest) {
        rest = after;
    }

    let rest = strip_prefix_ignore_case(rest, "re")
        .or_else(|| strip_prefix_ignore_case(rest, "fwd"))
        .or_else(|| strip_prefix_ignore_case(rest, "fw"))?;
    let rest = rest.trim_start_matches(' ');
    let rest = strip_blob(rest).unwrap_or(rest);
    rest.strip_prefix(':')
}

/// Strips one `subj-blob`: a bracketed text holding no bracket, and the
/// blanks after it.
fn strip_blob(s: &str) -> Option<&str> {
    let inner = s.strip_prefix('[')?;
    let end = inner.find(['[', ']'])?;
    let rest = inner[end..].strip_prefix(']')?;
    Some(rest.trim_start_matches(' '))
}

fn strip_prefix_ignore_case<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    let head = s.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix)
        .then(|| &s[prefix.len()..])
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;

    use crate::email::search::sort::eval::*;

    fn envelope(date: &str) -> Envelope {
        Envelope {
            id: String::from("1"),
            message_id: None,
            in_reply_to: Vec::new(),
            flags: Default::default(),
            subject: String::from("Release notes"),
            from: Vec::new(),
            to: Vec::new(),
            date: DateTime::parse_from_rfc3339(date).ok(),
            size: 1024,
            has_attachment: None,
        }
    }

    fn sorted(mut envelopes: Vec<Envelope>, kind: SearchEmailsSorterKind) -> Vec<String> {
        let sort = [SearchEmailsSorter(kind, SearchEmailsSorterOrder::Ascending)];
        sort_envelopes(&mut envelopes, Some(&sort));
        envelopes.into_iter().map(|e| e.id).collect()
    }

    fn with_subject(id: &str, subject: &str) -> Envelope {
        Envelope {
            id: id.into(),
            subject: subject.into(),
            ..envelope("2026-01-01T00:00:00+00:00")
        }
    }

    fn with_from(id: &str, name: Option<&str>, email: &str) -> Envelope {
        Envelope {
            id: id.into(),
            from: vec![Address {
                name: name.map(String::from),
                email: email.into(),
            }],
            ..envelope("2026-01-01T00:00:00+00:00")
        }
    }

    #[test]
    fn sort_by_date_descending_returns_newer_first() {
        let older = envelope("2026-01-01T00:00:00+00:00");
        let newer = envelope("2026-05-15T10:00:00+00:00");
        let sort = vec![SearchEmailsSorter(
            SearchEmailsSorterKind::Date,
            SearchEmailsSorterOrder::Descending,
        )];
        assert_eq!(compare_with(&newer, &older, &sort), Ordering::Less);
    }

    #[test]
    fn subjects_sort_on_their_base_ignoring_ascii_case() {
        let envelopes = vec![
            with_subject("1", "Re: Plain hello"),
            with_subject("2", "broken"),
            with_subject("3", "Café menu"),
            with_subject("4", "Plain hello"),
        ];

        assert_eq!(
            sorted(envelopes, SearchEmailsSorterKind::Subject),
            ["2", "3", "1", "4"],
        );
    }

    #[test]
    fn addresses_sort_on_the_first_mailbox_not_the_name() {
        let envelopes = vec![
            with_from("1", Some("Alice"), "zed@example.org"),
            with_from("2", Some("Zoe"), "Bob@example.org"),
            with_from("3", None, "carol@example.org"),
        ];

        assert_eq!(
            sorted(envelopes, SearchEmailsSorterKind::From),
            ["2", "3", "1"],
        );
    }

    #[test]
    fn the_base_subject_strips_what_rfc_5256_strips() {
        for (subject, base) in [
            ("Re: hello", "hello"),
            ("RE:hello", "hello"),
            ("Fwd: Re: Fw: hello", "hello"),
            ("re [list]: hello", "hello"),
            ("[list] Re: hello", "hello"),
            ("[list] hello", "hello"),
            ("[list]", "[list]"),
            ("hello (fwd) (FWD)  ", "hello"),
            ("[Fwd: Re: hello]", "hello"),
            ("  spaced \t out  ", "spaced out"),
            ("Réunion: Re: x", "Réunion: Re: x"),
            ("Ready: set", "Ready: set"),
        ] {
            assert_eq!(base_subject(subject), base, "{subject}");
        }
    }
}
