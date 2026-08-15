//! IMAP adapter for the shared cross-protocol client.
//!
//! Thin glue over [`ImapClient`], which already wraps io_imap's
//! high-level session (`select`, `fetch`, `store`, `copy`, `move`,
//! `append`, `list`, `status`). Each method takes and returns the
//! CLI's shared [`crate::email`] types; the only real work is
//! converting between those and io_imap's wire types, adapted from the
//! retired io-email IMAP drivers.

use io_imap::client::ImapClient as _;
use std::{
    collections::{BTreeMap, BTreeSet},
    num::NonZeroU32,
    str::from_utf8,
};

use anyhow::{Result, anyhow, bail};
use chrono::{DateTime, FixedOffset};
use io_imap::{
    rfc3501::{
        append::ImapMessageAppendOptions,
        copy::{ImapCopyUid, ImapMessageCopyOptions},
        fetch::ImapMessageFetchOptions,
        search::ImapMessageSearchOptions,
        select::ImapMailboxSelectOptions,
        store::ImapMessageStoreOptions,
    },
    rfc5256::sort::ImapMessageSortOptions,
    rfc6851::r#move::ImapMessageMoveOptions,
    types::{
        body::BodyStructure,
        core::{AString, Atom, Vec1},
        datetime::NaiveDate as ImapNaiveDate,
        envelope::Address as ImapAddress,
        extensions::sort::{SortCriterion, SortKey},
        fetch::{MacroOrMessageDataItemNames, MessageDataItem, MessageDataItemName},
        flag::{Flag as ImapFlag, FlagFetch, FlagNameAttribute, StoreType},
        mailbox::{ListMailbox, Mailbox as ImapMailbox},
        search::SearchKey,
        sequence::SequenceSet,
        status::{StatusDataItem, StatusDataItemName},
    },
};
use mail_parser::MessageParser;
use rfc2047_decoder::{Decoder, RecoverStrategy};

use crate::{
    email::{
        address::Address,
        envelope::{Envelope, normalize_message_id},
        flag::{Flag, FlagOp, IanaFlag},
        mailbox::Mailbox,
        search::{
            filter::query::SearchEmailsFilterQuery,
            query::SearchEmailsQuery,
            sort::query::{SearchEmailsSorter, SearchEmailsSorterKind, SearchEmailsSorterOrder},
        },
    },
    imap::client::ImapClient,
};

impl ImapClient {
    /// Lists every selectable mailbox. With `with_counts`, follows each
    /// row with a STATUS to populate totals and unread counts.
    pub fn list_mailboxes(&mut self, with_counts: bool) -> Result<Vec<Mailbox>> {
        let reference: ImapMailbox<'static> = ""
            .try_into()
            .map_err(|_| anyhow!("Invalid IMAP list reference"))?;
        let pattern: ListMailbox<'static> = "*"
            .try_into()
            .map_err(|_| anyhow!("Invalid IMAP list pattern"))?;

        let rows = self.list(reference, pattern)?;

        let mut mailboxes: Vec<Mailbox> = rows
            .into_iter()
            .filter(is_selectable)
            .map(mailbox_from)
            .collect();

        if with_counts {
            for mailbox in &mut mailboxes {
                let mbox = parse_mailbox(&mailbox.id)?;
                let items = self.status(
                    mbox,
                    vec![StatusDataItemName::Messages, StatusDataItemName::Unseen].into(),
                )?;
                apply_status(mailbox, items);
            }
        }

        Ok(mailboxes)
    }

    /// Lists envelopes from `mailbox`, most recent first. `page = None`
    /// and `page_size = None` fetch the whole mailbox.
    pub fn list_envelopes(
        &mut self,
        mailbox: &str,
        page: Option<u32>,
        page_size: Option<u32>,
        with_attachment: bool,
    ) -> Result<Vec<Envelope>> {
        let mbox = parse_mailbox(mailbox)?;
        let select = self.select(mbox, ImapMailboxSelectOptions::default())?;
        let exists = select.exists.unwrap_or(0);

        let Some(window) = compute_window(exists, page, page_size) else {
            return Ok(Vec::new());
        };
        let sequence_set: SequenceSet = window
            .as_str()
            .try_into()
            .map_err(|_| anyhow!("Invalid IMAP sequence-set window `{window}`"))?;

        let data = self.fetch(
            sequence_set,
            build_item_names(with_attachment),
            ImapMessageFetchOptions::default(),
        )?;

        let envelopes = data
            .into_iter()
            .rev()
            .map(|(seq, items)| envelope_from(seq.get(), items.into_inner()))
            .collect();

        Ok(envelopes)
    }

    /// Searches envelopes in `mailbox`: SELECT, then a UID SORT (RFC 5256,
    /// with client-side fallback) constrained by the translated query,
    /// paginated before a UID FETCH reordered back to the SORT order.
    pub fn search_envelopes(
        &mut self,
        mailbox: &str,
        query: Option<&SearchEmailsQuery>,
        page: Option<u32>,
        page_size: Option<u32>,
        with_attachment: bool,
    ) -> Result<Vec<Envelope>> {
        let mbox = parse_mailbox(mailbox)?;
        let select = self.select(mbox, ImapMailboxSelectOptions::default())?;
        if select.exists.unwrap_or(0) == 0 {
            return Ok(Vec::new());
        }

        let search_criteria = search_keys(query.and_then(|q| q.filter.as_ref()))?;
        let sort_criteria = sort_criteria(query.and_then(|q| q.sort.as_deref()));
        let fallback = self.sort_fallback();

        let uids = self.sort(
            sort_criteria,
            search_criteria,
            ImapMessageSortOptions {
                uid: true,
                fallback,
            },
        )?;
        if uids.is_empty() {
            return Ok(Vec::new());
        }

        let page_uids = paginate_uids(&uids, page, page_size);
        if page_uids.is_empty() {
            return Ok(Vec::new());
        }
        let uid_str = page_uids
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let sequence_set: SequenceSet = uid_str
            .as_str()
            .try_into()
            .map_err(|_| anyhow!("Invalid IMAP UID set `{uid_str}`"))?;

        let data = self.fetch(
            sequence_set,
            build_item_names(with_attachment),
            ImapMessageFetchOptions {
                uid: true,
                ..Default::default()
            },
        )?;

        Ok(reorder_envelopes(data, &page_uids))
    }

    /// Adds, sets, or removes `flags` on a UID set in `mailbox`.
    pub fn store_flags(
        &mut self,
        mailbox: &str,
        ids: &[&str],
        flags: &[Flag],
        op: FlagOp,
    ) -> Result<()> {
        let mbox = parse_mailbox(mailbox)?;
        let sequence_set = parse_uids(ids)?;
        let imap_flags: Vec<ImapFlag<'static>> = flags.iter().map(flag_from).collect();
        let kind = match op {
            FlagOp::Add => StoreType::Add,
            FlagOp::Set => StoreType::Replace,
            FlagOp::Remove => StoreType::Remove,
        };

        self.select(mbox, ImapMailboxSelectOptions::default())?;
        self.store(
            sequence_set,
            kind,
            imap_flags,
            ImapMessageStoreOptions { uid: true },
        )?;

        Ok(())
    }

    /// Fetches one message's raw RFC 5322 bytes without flipping
    /// `\Seen` (BODY.PEEK[]).
    pub fn get_message(&mut self, mailbox: &str, id: &str, seen: bool) -> Result<Vec<u8>> {
        let mbox = parse_mailbox(mailbox)?;
        let sequence_set = parse_uids(&[id])?;

        self.select(mbox, ImapMailboxSelectOptions::default())?;

        // `BODY[]` sets `\Seen` server-side as a side effect of the fetch,
        // so `--seen` costs no extra round trip; `BODY.PEEK[]` leaves the
        // flag untouched. The mailbox is always SELECTed read-write.
        let item_names =
            MacroOrMessageDataItemNames::MessageDataItemNames(vec![MessageDataItemName::BodyExt {
                section: None,
                partial: None,
                peek: !seen,
            }]);
        let data = self.fetch(
            sequence_set,
            item_names,
            ImapMessageFetchOptions {
                uid: true,
                ..Default::default()
            },
        )?;

        data.into_values()
            .flat_map(|items| items.into_inner().into_iter())
            .find_map(|item| match item {
                MessageDataItem::BodyExt { data, .. } => data.0.map(|d| d.as_ref().to_vec()),
                _ => None,
            })
            .ok_or_else(|| anyhow!("FETCH returned no body for the requested message"))
    }

    /// Appends `raw` to `mailbox` with `flags`, returning the appended
    /// UID (UIDPLUS APPENDUID, else a UID SEARCH on Message-ID).
    pub fn add_message(&mut self, mailbox: &str, flags: &[Flag], raw: Vec<u8>) -> Result<String> {
        let mbox = parse_mailbox(mailbox)?;
        let imap_flags: Vec<ImapFlag<'static>> = flags.iter().map(flag_from).collect();

        let (_, appenduid) = self.append(
            mbox.clone(),
            &raw,
            ImapMessageAppendOptions {
                flags: imap_flags,
                date: None,
                non_sync: false,
            },
        )?;

        if let Some((_, uid)) = appenduid {
            return Ok(uid.to_string());
        }

        // No UIDPLUS: recover the UID via SELECT + UID SEARCH on the
        // message's own Message-ID (needs one on the message).
        let message_id = MessageParser::default()
            .parse_headers(&raw)
            .and_then(|parsed| parsed.message_id().map(str::to_string))
            .filter(|id| !id.is_empty());
        let Some(message_id) = message_id else {
            bail!(
                "Cannot resolve appended UID: server lacks UIDPLUS and message has no Message-ID"
            );
        };

        self.select(mbox, ImapMailboxSelectOptions::default())?;

        let field =
            AString::try_from("Message-ID").map_err(|_| anyhow!("Invalid IMAP search header"))?;
        let value = AString::try_from(message_id)
            .map_err(|_| anyhow!("Invalid IMAP search Message-ID value"))?;
        let criteria = Vec1::from(SearchKey::Header(field, value));
        let uids = self.search(criteria, ImapMessageSearchOptions { uid: true })?;

        uids.into_iter()
            .max()
            .map(|uid| uid.to_string())
            .ok_or_else(|| anyhow!("Fallback UID search returned no match"))
    }

    /// Copies a UID set from `from` to `to`.
    pub fn copy_messages(&mut self, from: &str, to: &str, ids: &[&str]) -> Result<usize> {
        let source = parse_mailbox(from)?;
        let target = parse_mailbox(to)?;
        let sequence_set = parse_uids(ids)?;

        self.select(source, ImapMailboxSelectOptions::default())?;
        let copy_uid = self.copy(sequence_set, target, ImapMessageCopyOptions { uid: true })?;

        Ok(self.copied_count(copy_uid, ids.len()))
    }

    /// Moves a UID set from `from` to `to` (RFC 6851).
    pub fn move_messages(&mut self, from: &str, to: &str, ids: &[&str]) -> Result<usize> {
        let source = parse_mailbox(from)?;
        let target = parse_mailbox(to)?;
        let sequence_set = parse_uids(ids)?;

        self.select(source, ImapMailboxSelectOptions::default())?;
        let copy_uid = self.r#move(sequence_set, target, ImapMessageMoveOptions { uid: true })?;

        Ok(self.copied_count(copy_uid, ids.len()))
    }

    /// Permanently deletes `ids` from `mailbox` (the trash): flags them
    /// `\Deleted`, then `UID EXPUNGE`s exactly those UIDs when the server
    /// advertises UIDPLUS (RFC 4315), leaving any other `\Deleted`
    /// message untouched. Returns `true` when the messages were
    /// physically removed, `false` when only flagged (no UIDPLUS).
    pub fn delete_messages(&mut self, mailbox: &str, ids: &[&str]) -> Result<bool> {
        let mbox = parse_mailbox(mailbox)?;

        self.select(mbox, ImapMailboxSelectOptions::default())?;
        self.store(
            parse_uids(ids)?,
            StoreType::Add,
            vec![ImapFlag::Deleted],
            ImapMessageStoreOptions { uid: true },
        )?;

        if self.supports_uidplus() {
            self.uid_expunge(parse_uids(ids)?)?;
            return Ok(true);
        }

        Ok(false)
    }

    /// Number of messages a UID COPY/MOVE actually affected. UIDPLUS
    /// servers report a `COPYUID` whose source set lists exactly the
    /// affected UIDs, so it is authoritative — including its absence,
    /// which means nothing matched (a stale-UID no-op). Servers without
    /// UIDPLUS give no such feedback, so fall back to the requested
    /// count rather than under-report a real copy as zero.
    fn copied_count(&self, copy_uid: ImapCopyUid, requested: usize) -> usize {
        match copy_uid {
            Some((_, source_uids, _)) => source_uids.len(),
            None if self.supports_uidplus() => 0,
            None => requested,
        }
    }
}

/// One IMAP LIST row (mailbox, delimiter, attributes).
type ListRow = (
    ImapMailbox<'static>,
    Option<io_imap::types::core::QuotedChar>,
    Vec<FlagNameAttribute<'static>>,
);

/// Drops `\Noselect` containers (RFC 3501 §6.3.8): they cannot hold
/// messages and would error out on any later shared-API op.
fn is_selectable(row: &ListRow) -> bool {
    !row.2.contains(&FlagNameAttribute::Noselect)
}

/// Converts one IMAP LIST row into the shared [`Mailbox`] shape.
fn mailbox_from(row: ListRow) -> Mailbox {
    let name = match row.0 {
        ImapMailbox::Inbox => "Inbox".to_string(),
        ImapMailbox::Other(other) => String::from_utf8_lossy(other.inner().as_ref()).into_owned(),
    };

    Mailbox {
        id: name.clone(),
        name,
        total: None,
        unread: None,
    }
}

/// Folds a STATUS response into the matching mailbox row.
fn apply_status(mailbox: &mut Mailbox, items: Vec<StatusDataItem>) {
    for item in items {
        match item {
            StatusDataItem::Messages(n) => mailbox.total = Some(u64::from(n)),
            StatusDataItem::Unseen(n) => mailbox.unread = Some(u64::from(n)),
            _ => {}
        }
    }
}

/// FETCH item-name list: UID + FLAGS + ENVELOPE + RFC822.SIZE, plus
/// BODYSTRUCTURE when `with_attachment` is set.
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

/// Sequence-set string for `(page, page_size)` against `exists`, or
/// `None` for an empty window. Page 1 is the most recent window.
fn compute_window(exists: u32, page: Option<u32>, page_size: Option<u32>) -> Option<String> {
    if exists == 0 {
        return None;
    }
    let page = page.unwrap_or(1).max(1);
    let Some(size) = page_size else {
        return Some("1:*".to_string());
    };
    if size == 0 {
        return None;
    }
    let skip = (page - 1).saturating_mul(size);
    if skip >= exists {
        return None;
    }
    let end = exists - skip;
    let start = end.saturating_sub(size - 1).max(1);
    Some(format!("{start}:{end}"))
}

/// Folds one FETCH row into a shared [`Envelope`].
fn envelope_from(seq: u32, items: Vec<MessageDataItem<'static>>) -> Envelope {
    let mut id = String::new();
    let mut message_id: Option<String> = None;
    let mut flags = BTreeSet::new();
    let mut subject = String::new();
    let mut from = Vec::new();
    let mut to = Vec::new();
    let mut date: Option<DateTime<FixedOffset>> = None;
    let mut size: u64 = 0;
    let mut has_attachment: Option<bool> = None;

    for item in items {
        match item {
            MessageDataItem::Uid(uid) => id = uid.get().to_string(),
            MessageDataItem::Flags(fs) => {
                flags = fs.into_iter().filter_map(flag_from_fetch).collect();
            }
            MessageDataItem::Envelope(env) => {
                if let Some(s) = env.subject.into_option() {
                    subject = decode_mime_bytes(s.as_ref());
                }
                if let Some(d) = env.date.into_option() {
                    date = parse_rfc2822_date(&bytes_to_string(d.as_ref()));
                }
                if let Some(m) = env.message_id.into_option() {
                    message_id = normalize_message_id(&bytes_to_string(m.as_ref()));
                }
                from = env.from.iter().map(address_from).collect();
                to = env.to.iter().map(address_from).collect();
            }
            MessageDataItem::Rfc822Size(n) => size = u64::from(n),
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
        BodyStructure::Single { extension_data, .. } => extension_data
            .as_ref()
            .and_then(|ext| ext.tail.as_ref())
            .and_then(|disposition| disposition.disposition.as_ref())
            .map(|(kind, _)| kind.as_ref().eq_ignore_ascii_case(b"attachment"))
            .unwrap_or(false),
        BodyStructure::Multi { bodies, .. } => {
            bodies.as_ref().iter().any(body_structure_has_attachment)
        }
    }
}

/// Maps a shared [`Flag`] to its IMAP wire counterpart. IANA flags
/// become the matching system flag; custom keywords pass through as
/// Keyword atoms, with a sanitised fallback for non-atom-safe input.
fn flag_from(flag: &Flag) -> ImapFlag<'static> {
    match flag.iana() {
        Some(IanaFlag::Seen) => ImapFlag::Seen,
        Some(IanaFlag::Answered) => ImapFlag::Answered,
        Some(IanaFlag::Flagged) => ImapFlag::Flagged,
        Some(IanaFlag::Draft) => ImapFlag::Draft,
        Some(IanaFlag::Deleted) => ImapFlag::Deleted,
        Some(_) => ImapFlag::keyword(
            Atom::try_from(String::from(flag.raw()))
                .expect("canonical IANA keyword is a valid IMAP atom"),
        ),
        None => match Atom::try_from(String::from(flag.raw())) {
            Ok(atom) => ImapFlag::keyword(atom),
            Err(_) => ImapFlag::keyword(
                Atom::try_from(sanitise_atom(flag.raw()))
                    .expect("sanitised atom contains only atom-safe ASCII"),
            ),
        },
    }
}

/// Replaces every non-atom-safe byte with `_` so a keyword with spaces,
/// controls or `()<>{}` survives IMAP STORE.
fn sanitise_atom(raw: &str) -> String {
    raw.chars()
        .map(|c| {
            if c.is_ascii()
                && !c.is_control()
                && !matches!(
                    c,
                    ' ' | '(' | ')' | '{' | '%' | '*' | '"' | '\\' | ']' | '\x7f'
                )
            {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Parses a shared mailbox name into an IMAP Mailbox token.
fn parse_mailbox(name: &str) -> Result<ImapMailbox<'static>> {
    String::from(name)
        .try_into()
        .map_err(|_| anyhow!("Invalid IMAP mailbox `{name}`"))
}

/// Parses stringified UIDs into an IMAP [`SequenceSet`].
fn parse_uids(ids: &[&str]) -> Result<SequenceSet> {
    if ids.is_empty() {
        bail!("Empty UID set");
    }

    let uids: Vec<std::num::NonZeroU32> = ids
        .iter()
        .map(|s| {
            s.parse::<std::num::NonZeroU32>()
                .map_err(|_| anyhow!("Invalid message UID `{s}`"))
        })
        .collect::<Result<_>>()?;

    SequenceSet::try_from(uids).map_err(|_| anyhow!("Invalid UID set"))
}

fn parse_rfc2822_date(raw: &str) -> Option<DateTime<FixedOffset>> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    // `chrono` validates the optional leading day-of-week against the
    // date and rejects the whole timestamp when they disagree (a
    // surprising number of senders get the weekday wrong). The weekday
    // is redundant, so on failure retry without it.
    DateTime::parse_from_rfc2822(trimmed)
        .or_else(|_| DateTime::parse_from_rfc2822(strip_weekday(trimmed)))
        .ok()
}

/// Drops a leading `Dow, ` day-of-week token (e.g. `Thu, `) from an
/// RFC 2822 date, leaving the unambiguous `DD Mon YYYY …` remainder
/// that `chrono` parses without a weekday check. Returns the input
/// untouched when it has no such prefix.
fn strip_weekday(date: &str) -> &str {
    match date.split_once(", ") {
        Some((dow, rest)) if dow.len() == 3 && dow.bytes().all(|b| b.is_ascii_alphabetic()) => rest,
        _ => date,
    }
}

fn bytes_to_string(bytes: &[u8]) -> String {
    from_utf8(bytes).map(str::to_string).unwrap_or_else(|_| {
        let mut out = String::with_capacity(bytes.len());
        for b in bytes {
            out.push(*b as char);
        }
        out
    })
}

/// Decodes RFC 2047 MIME-encoded words from IMAP ENVELOPE strings;
/// falls back to [`bytes_to_string`] on malformed input.
fn decode_mime_bytes(bytes: &[u8]) -> String {
    let decoder = Decoder::new().too_long_encoded_word_strategy(RecoverStrategy::Decode);
    decoder
        .decode(bytes)
        .unwrap_or_else(|_| bytes_to_string(bytes))
}

/// SEARCH key list for `filter`, defaulting to ALL.
fn search_keys(filter: Option<&SearchEmailsFilterQuery>) -> Result<Vec1<SearchKey<'static>>> {
    let key = match filter {
        None => SearchKey::All,
        Some(filter) => convert_filter(filter)?,
    };
    Ok(Vec1::from(key))
}

/// SORT criterion list for `sort`, defaulting to REVERSE DATE.
fn sort_criteria(sort: Option<&[SearchEmailsSorter]>) -> Vec1<SortCriterion> {
    let criteria: Vec<SortCriterion> = match sort {
        Some(chain) if !chain.is_empty() => chain.iter().map(convert_sorter).collect(),
        _ => vec![SortCriterion {
            reverse: true,
            key: SortKey::Date,
        }],
    };

    Vec1::try_from(criteria).expect("non-empty by construction")
}

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

        // Date(D) maps onto SENTON (Date: header on day D).
        Q::Date(date) => SearchKey::SentOn(imap_date(*date)?),

        // AfterDate(D) is strict "> D"; SENTSINCE is ">=", so bump one day.
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
                    .map_err(|_| anyhow!("Invalid IMAP keyword `{}`", flag.raw()))?,
            ),
        },
    })
}

fn convert_sorter(sorter: &SearchEmailsSorter) -> SortCriterion {
    let SearchEmailsSorter(kind, order) = sorter;

    let key = match kind {
        SearchEmailsSorterKind::Date => SortKey::Date,
        SearchEmailsSorterKind::From => SortKey::From,
        SearchEmailsSorterKind::To => SortKey::To,
        SearchEmailsSorterKind::Subject => SortKey::Subject,
    };

    SortCriterion {
        reverse: matches!(order, SearchEmailsSorterOrder::Descending),
        key,
    }
}

fn astring(pattern: &str) -> Result<AString<'static>> {
    AString::try_from(String::from(pattern))
        .map_err(|_| anyhow!("Invalid IMAP search pattern `{pattern}`"))
}

fn imap_date(date: chrono::NaiveDate) -> Result<ImapNaiveDate> {
    ImapNaiveDate::try_from(date).map_err(|_| anyhow!("Invalid IMAP date `{date}`"))
}

/// Slices `uids` for `(page, page_size)`, preserving SORT order.
fn paginate_uids(uids: &[NonZeroU32], page: Option<u32>, page_size: Option<u32>) -> Vec<u32> {
    let total = uids.len();
    let size = page_size.map(|n| n as usize);
    let start = ((page.unwrap_or(1).max(1) - 1) as usize).saturating_mul(size.unwrap_or(0));

    if start >= total {
        return Vec::new();
    }

    let end = match size {
        Some(n) => start.saturating_add(n).min(total),
        None => total,
    };

    uids[start..end].iter().map(|u| u.get()).collect()
}

/// Reorders the FETCH response into the requested UID order, dropping
/// UIDs the server skipped.
fn reorder_envelopes(
    data: BTreeMap<NonZeroU32, Vec1<MessageDataItem<'static>>>,
    order: &[u32],
) -> Vec<Envelope> {
    let by_uid: BTreeMap<u32, Envelope> = data
        .into_iter()
        .map(|(seq, items)| {
            let items = items.into_inner();
            let uid = items.iter().find_map(|item| match item {
                MessageDataItem::Uid(u) => Some(u.get()),
                _ => None,
            });
            let envelope = envelope_from(seq.get(), items);
            (uid.unwrap_or(seq.get()), envelope)
        })
        .collect();

    order
        .iter()
        .filter_map(|u| by_uid.get(u).cloned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::parse_rfc2822_date;

    #[test]
    fn parses_a_well_formed_date() {
        let date = parse_rfc2822_date("Fri, 17 Jul 2026 10:00:00 +0000").unwrap();
        assert_eq!(date.to_rfc3339(), "2026-07-17T10:00:00+00:00");
    }

    #[test]
    fn parses_despite_a_wrong_weekday() {
        // 2026-07-17 is a Friday; `Thu` is wrong but must not void the date.
        let date = parse_rfc2822_date("Thu, 17 Jul 2026 10:00:00 +0000").unwrap();
        assert_eq!(date.to_rfc3339(), "2026-07-17T10:00:00+00:00");
    }

    #[test]
    fn parses_without_a_weekday() {
        let date = parse_rfc2822_date("17 Jul 2026 10:00:00 +0000").unwrap();
        assert_eq!(date.to_rfc3339(), "2026-07-17T10:00:00+00:00");
    }

    #[test]
    fn rejects_empty_and_garbage() {
        assert!(parse_rfc2822_date("   ").is_none());
        assert!(parse_rfc2822_date("not a date").is_none());
    }
}
