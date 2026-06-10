//! UID SEARCH fallback for IMAP servers without the RFC 5256 SORT
//! extension (notably Gmail, which rejects `UID SORT` with `BAD
//! Unknown command`).
//!
//! The shared `envelope search` command normally goes through
//! io-email's `ImapEnvelopeSearch` coroutine, which always issues
//! `UID SORT` — even when the query carries no `order by` clause
//! (the sort then defaults to `REVERSE DATE`). On servers that do
//! not advertise the SORT capability this fails wholesale.
//!
//! This module reruns the same filter as a plain `UID SEARCH`
//! (RFC 3501) on the already-authenticated session:
//!
//! - without `order by`, matching UIDs are ordered newest-first
//!   (descending UID, i.e. mailbox arrival order — an approximation
//!   of the `REVERSE DATE` default) and only the requested page is
//!   fetched;
//! - with `order by`, every matching envelope is fetched (in chunks)
//!   and sorted client-side before pagination, mirroring what v1 did.
//!
//! The FETCH→[`Envelope`] conversion mirrors io-email's private
//! `envelope_from`; the durable home for this whole fallback is
//! io-email's search coroutine itself, at which point this module
//! can be deleted.

use std::{cmp::Ordering, collections::BTreeMap, num::NonZeroU32, str::from_utf8};

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, FixedOffset};
use io_email::address::Address;
use io_email::{
    client::EmailClientStdError,
    envelope::{
        imap::search::ImapEnvelopeSearchError,
        types::{Envelope, normalize_message_id},
    },
    flag::types::{Flag, IanaFlag},
    imap::{
        client::{ImapClientError, ImapClientStd},
        convert::parse_mailbox,
    },
    search::{
        filter::query::SearchEmailsFilterQuery,
        query::SearchEmailsQuery,
        sort::query::{SearchEmailsSorter, SearchEmailsSorterKind, SearchEmailsSorterOrder},
    },
};
use io_imap::{
    rfc3501::{
        fetch::{ImapMessageFetch, ImapMessageFetchOptions},
        search::{ImapMessageSearch, ImapMessageSearchOptions},
        select::{ImapMailboxSelect, ImapMailboxSelectOptions},
    },
    rfc5256::sort::ImapMailboxSortError,
    types::{
        body::BodyStructure,
        core::{AString, Atom, Vec1},
        datetime::NaiveDate as ImapNaiveDate,
        envelope::Address as ImapAddress,
        fetch::{MacroOrMessageDataItemNames, MessageDataItem, MessageDataItemName},
        flag::FlagFetch,
        search::SearchKey,
        sequence::SequenceSet,
    },
};
use log::debug;
use rfc2047_decoder::{Decoder, RecoverStrategy};

/// UIDs per FETCH round-trip when the whole match set is needed for
/// client-side sorting.
const FETCH_CHUNK: usize = 500;

/// Whether `err` is the server actively refusing `UID SORT` (tagged
/// NO/BAD), i.e. the case this fallback exists for. Transport-level
/// failures keep bubbling up unchanged.
pub fn applies(err: &EmailClientStdError) -> bool {
    matches!(
        err,
        EmailClientStdError::Imap(ImapClientError::EnvelopeSearch(
            ImapEnvelopeSearchError::Sort(
                ImapMailboxSortError::No(_) | ImapMailboxSortError::Bad(_)
            )
        ))
    )
}

/// Reruns the search as `SELECT` + `UID SEARCH` + paged `UID FETCH`
/// on the same session. See the module docs for ordering semantics.
pub fn search(
    imap: &mut ImapClientStd,
    mailbox: &str,
    query: Option<&SearchEmailsQuery>,
    page: Option<u32>,
    page_size: Option<u32>,
    with_attachment: bool,
) -> Result<Vec<Envelope>> {
    debug!("server lacks SORT: falling back to UID SEARCH for mailbox {mailbox}");

    let mbox = parse_mailbox(mailbox).map_err(|err| anyhow!("invalid IMAP mailbox `{}`", err.0))?;
    imap.inner
        .run(ImapMailboxSelect::new(
            mbox,
            ImapMailboxSelectOptions::default(),
        ))
        .context("cannot select mailbox for UID SEARCH fallback")?;

    let criteria = match query.and_then(|q| q.filter.as_ref()) {
        None => Vec1::from(SearchKey::All),
        Some(filter) => Vec1::from(convert_filter(filter)?),
    };
    let mut uids: Vec<u32> = imap
        .inner
        .run(ImapMessageSearch::new(
            criteria,
            ImapMessageSearchOptions { uid: true },
        ))
        .context("cannot run UID SEARCH fallback")?
        .into_iter()
        .map(NonZeroU32::get)
        .collect();

    let sorters = query.and_then(|q| q.sort.as_deref()).unwrap_or_default();

    if sorters.is_empty() {
        // Newest-first by arrival order, fetch the requested page only.
        uids.sort_unstable_by(|a, b| b.cmp(a));
        let uid_page = paginate(&uids, page, page_size);
        let envelopes = fetch_envelopes(imap, &uid_page, with_attachment)?;
        Ok(reorder(envelopes, &uid_page))
    } else {
        // Client-side sort needs every matching envelope.
        let mut envelopes = Vec::with_capacity(uids.len());
        for chunk in uids.chunks(FETCH_CHUNK) {
            envelopes.extend(fetch_envelopes(imap, chunk, with_attachment)?.into_values());
        }
        envelopes.sort_by(|a, b| compare(a, b, sorters));
        let page = paginate(&envelopes, page, page_size);
        Ok(page)
    }
}

/// Slices `items` for `(page, page_size)`; `None` page size means
/// everything.
fn paginate<T: Clone>(items: &[T], page: Option<u32>, page_size: Option<u32>) -> Vec<T> {
    let Some(size) = page_size.map(|n| n as usize).filter(|n| *n > 0) else {
        return items.to_vec();
    };
    let start = ((page.unwrap_or(1).max(1) - 1) as usize).saturating_mul(size);
    if start >= items.len() {
        return Vec::new();
    }
    let end = start.saturating_add(size).min(items.len());
    items[start..end].to_vec()
}

/// `UID FETCH` for the given UIDs, keyed back by UID.
fn fetch_envelopes(
    imap: &mut ImapClientStd,
    uids: &[u32],
    with_attachment: bool,
) -> Result<BTreeMap<u32, Envelope>> {
    if uids.is_empty() {
        return Ok(BTreeMap::new());
    }

    let uid_str = uids
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let sequence_set: SequenceSet = uid_str
        .as_str()
        .try_into()
        .map_err(|_| anyhow!("invalid IMAP UID set `{uid_str}`"))?;

    let data = imap
        .inner
        .run(ImapMessageFetch::new(
            sequence_set,
            build_item_names(with_attachment),
            ImapMessageFetchOptions {
                uid: true,
                ..Default::default()
            },
        ))
        .context("cannot fetch envelopes for UID SEARCH fallback")?;

    Ok(data
        .into_iter()
        .map(|(seq, items)| {
            let items = items.into_inner();
            let uid = items.iter().find_map(|item| match item {
                MessageDataItem::Uid(u) => Some(u.get()),
                _ => None,
            });
            let env = envelope_from(seq.get(), items);
            (uid.unwrap_or_else(|| seq.get()), env)
        })
        .collect())
}

/// Reorders fetched envelopes into the requested UID order, dropping
/// UIDs the server skipped.
fn reorder(mut by_uid: BTreeMap<u32, Envelope>, order: &[u32]) -> Vec<Envelope> {
    order.iter().filter_map(|u| by_uid.remove(u)).collect()
}

/// Left-to-right sorter chain comparison; ties fall through to the
/// next sorter.
fn compare(a: &Envelope, b: &Envelope, sorters: &[SearchEmailsSorter]) -> Ordering {
    for SearchEmailsSorter(kind, order) in sorters {
        let ord = match kind {
            SearchEmailsSorterKind::Date => a.date.cmp(&b.date),
            SearchEmailsSorterKind::From => address_key(&a.from).cmp(&address_key(&b.from)),
            SearchEmailsSorterKind::To => address_key(&a.to).cmp(&address_key(&b.to)),
            SearchEmailsSorterKind::Subject => {
                a.subject.to_lowercase().cmp(&b.subject.to_lowercase())
            }
        };
        let ord = match order {
            SearchEmailsSorterOrder::Ascending => ord,
            SearchEmailsSorterOrder::Descending => ord.reverse(),
        };
        if ord != Ordering::Equal {
            return ord;
        }
    }
    Ordering::Equal
}

fn address_key(addresses: &[Address]) -> String {
    addresses
        .first()
        .map(|a| a.name.clone().unwrap_or_else(|| a.email.clone()))
        .unwrap_or_default()
        .to_lowercase()
}

/// Shared-DSL filter → RFC 3501 SEARCH keys. Mirrors io-email's
/// private `convert_filter` so the fallback matches exactly what the
/// SORT-based path would have matched.
fn convert_filter(filter: &SearchEmailsFilterQuery) -> Result<SearchKey<'static>> {
    use SearchEmailsFilterQuery as Q;

    Ok(match filter {
        Q::And(left, right) => {
            let keys = vec![convert_filter(left)?, convert_filter(right)?];
            SearchKey::And(Vec1::try_from(keys).expect("non-empty by construction"))
        }
        Q::Or(left, right) => SearchKey::Or(
            Box::new(convert_filter(left)?),
            Box::new(convert_filter(right)?),
        ),
        Q::Not(inner) => SearchKey::Not(Box::new(convert_filter(inner)?)),

        Q::Date(date) => SearchKey::SentOn(imap_date(*date)?),

        // AfterDate is strict "> D"; SENTSINCE is ">=", so bump by one
        // day (same as io-email).
        Q::AfterDate(date) => {
            let bumped = date.succ_opt().unwrap_or(*date);
            SearchKey::SentSince(imap_date(bumped)?)
        }

        Q::From(pattern) => SearchKey::From(astring(pattern)?),
        Q::To(pattern) => SearchKey::To(astring(pattern)?),
        Q::Subject(pattern) => SearchKey::Subject(astring(pattern)?),
        Q::Body(pattern) => SearchKey::Body(astring(pattern)?),

        Q::Flag(flag) => match flag.iana() {
            Some(IanaFlag::Seen) => SearchKey::Seen,
            Some(IanaFlag::Answered) => SearchKey::Answered,
            Some(IanaFlag::Flagged) => SearchKey::Flagged,
            Some(IanaFlag::Draft) => SearchKey::Draft,
            Some(IanaFlag::Deleted) => SearchKey::Deleted,
            _ => SearchKey::Keyword(
                Atom::try_from(String::from(flag.raw()))
                    .map_err(|_| anyhow!("invalid IMAP keyword `{}`", flag.raw()))?,
            ),
        },
    })
}

fn astring(pattern: &str) -> Result<AString<'static>> {
    AString::try_from(String::from(pattern))
        .map_err(|_| anyhow!("invalid IMAP search pattern `{pattern}`"))
}

fn imap_date(date: chrono::NaiveDate) -> Result<ImapNaiveDate> {
    ImapNaiveDate::try_from(date).map_err(|_| anyhow!("invalid IMAP date `{date}`"))
}

/// FETCH item-name list: UID + FLAGS + ENVELOPE + RFC822.SIZE, plus
/// BODYSTRUCTURE when `with_attachment` is set (same set io-email
/// fetches).
fn build_item_names(with_attachment: bool) -> MacroOrMessageDataItemNames<'static> {
    let mut names = vec![
        MessageDataItemName::Uid,
        MessageDataItemName::Flags,
        MessageDataItemName::Envelope,
        MessageDataItemName::Rfc822Size,
    ];
    if with_attachment {
        names.push(MessageDataItemName::BodyStructure);
    }
    MacroOrMessageDataItemNames::MessageDataItemNames(names)
}

/// Folds one FETCH row into a shared [`Envelope`]. Mirrors io-email's
/// private `envelope_from`.
fn envelope_from(seq: u32, items: Vec<MessageDataItem<'static>>) -> Envelope {
    let mut id = String::new();
    let mut message_id: Option<String> = None;
    let mut flags = std::collections::BTreeSet::new();
    let mut subject = String::new();
    let mut from = Vec::new();
    let mut to = Vec::new();
    let mut date: Option<DateTime<FixedOffset>> = None;
    let mut size: u64 = 0;
    let mut has_attachment: Option<bool> = None;

    for item in items {
        match item {
            MessageDataItem::Uid(uid) => {
                id = uid.get().to_string();
            }
            MessageDataItem::Flags(fs) => {
                flags = fs.into_iter().filter_map(flag_from_fetch).collect();
            }
            MessageDataItem::Envelope(env) => {
                if let Some(s) = env.subject.into_option() {
                    subject = decode_mime_bytes(s.as_ref());
                }
                if let Some(d) = env.date.into_option() {
                    let raw = bytes_to_string(d.as_ref());
                    date = DateTime::parse_from_rfc2822(raw.trim()).ok();
                }
                if let Some(m) = env.message_id.into_option() {
                    let raw = bytes_to_string(m.as_ref());
                    message_id = normalize_message_id(&raw);
                }
                from = env.from.iter().map(address_from).collect();
                to = env.to.iter().map(address_from).collect();
            }
            MessageDataItem::Rfc822Size(n) => {
                size = u64::from(n);
            }
            MessageDataItem::BodyStructure(structure) => {
                has_attachment = Some(body_structure_has_attachment(&structure));
            }
            _ => {}
        }
    }

    if id.is_empty() {
        id = seq.to_string();
    }

    Envelope {
        id,
        message_id,
        flags,
        subject,
        from,
        to,
        date,
        size,
        has_attachment,
    }
}

fn flag_from_fetch(fetch: FlagFetch<'_>) -> Option<Flag> {
    let FlagFetch::Flag(flag) = fetch else {
        return None;
    };
    Some(Flag::from_raw(flag.to_string()))
}

fn address_from(addr: &ImapAddress<'_>) -> Address {
    let name = addr
        .name
        .0
        .as_ref()
        .map(|s| decode_mime_bytes(s.as_ref()))
        .filter(|s| !s.is_empty());

    let mailbox = addr
        .mailbox
        .0
        .as_ref()
        .map(|s| bytes_to_string(s.as_ref()))
        .unwrap_or_default();

    let host = addr
        .host
        .0
        .as_ref()
        .map(|s| bytes_to_string(s.as_ref()))
        .unwrap_or_default();

    let email = if mailbox.is_empty() {
        host
    } else if host.is_empty() {
        mailbox
    } else {
        format!("{mailbox}@{host}")
    };

    Address { name, email }
}

fn body_structure_has_attachment(structure: &BodyStructure<'_>) -> bool {
    match structure {
        BodyStructure::Single { extension_data, .. } => {
            let Some(ext) = extension_data.as_ref() else {
                return false;
            };
            let Some(disposition) = ext.tail.as_ref() else {
                return false;
            };
            let Some((kind, _)) = disposition.disposition.as_ref() else {
                return false;
            };
            kind.as_ref().eq_ignore_ascii_case(b"attachment")
        }
        BodyStructure::Multi { bodies, .. } => {
            bodies.as_ref().iter().any(body_structure_has_attachment)
        }
    }
}

fn bytes_to_string(bytes: &[u8]) -> String {
    from_utf8(bytes)
        .map(ToString::to_string)
        .unwrap_or_else(|_| bytes.iter().map(|b| *b as char).collect())
}

/// Decodes RFC 2047 MIME-encoded words from IMAP ENVELOPE strings;
/// falls back to [`bytes_to_string`] on malformed input.
fn decode_mime_bytes(bytes: &[u8]) -> String {
    let decoder = Decoder::new().too_long_encoded_word_strategy(RecoverStrategy::Decode);
    decoder
        .decode(bytes)
        .unwrap_or_else(|_| bytes_to_string(bytes))
}
