//! # Search sort evaluation
//!
//! Client-side ordering of envelopes by the sort half of a shared search
//! query, for the backends that search locally and for Graph, which sorts
//! after searching.

use std::cmp::{Ordering, Reverse};

use crate::email::{
    address::Address,
    envelope::Envelope,
    search::sort::query::{SearchEmailsSorter, SearchEmailsSorterKind, SearchEmailsSorterOrder},
};

/// Sorts envelopes by the given chain, or by `Date:` descending when the
/// query carries no sort clause.
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
}
