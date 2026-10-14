//! # IMAP backend
//!
//! The IMAP adapter of the shared cross-protocol client, glue over the
//! io-imap session [`ImapClient`] already wraps.
//!
//! Each method takes and returns the CLI's own [`crate::email`] types, so
//! the work here is converting between those and io-imap's wire types.

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
use log::debug;
use mail_parser::MessageParser;
use rfc2047_decoder::{Decoder, RecoverStrategy};

use crate::{
    email::{
        address::Address,
        envelope::{Envelope, normalize_message_id, parse_message_ids},
        flag::{Flag, FlagOp, IanaFlag},
        mailbox::{Mailbox, MailboxRole},
        search::{
            filter::query::SearchEmailsFilterQuery,
            query::SearchEmailsQuery,
            sort::query::{SearchEmailsSorter, SearchEmailsSorterKind, SearchEmailsSorterOrder},
        },
    },
    imap::client::ImapClient,
};

impl ImapClient {
    /// Lists every selectable mailbox, following each with a `STATUS`
    /// when counts were asked for.
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

    /// Lists envelopes most recent first, the whole mailbox when neither
    /// page nor page size is given.
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

    /// Searches a mailbox with the shared query.
    ///
    /// A UID `SORT`, or its client-side fallback, is constrained by the
    /// translated query, paginated, then fetched and reordered back into
    /// the order the sort gave.
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

    /// Adds, sets or removes flags on a UID set.
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

    /// Fetches one message's raw RFC 5322 bytes, marking it seen only
    /// when asked.
    pub fn get_message(&mut self, mailbox: &str, id: &str, seen: bool) -> Result<Vec<u8>> {
        let mbox = parse_mailbox(mailbox)?;
        let sequence_set = parse_uids(&[id])?;

        self.select(mbox, ImapMailboxSelectOptions::default())?;

        // NOTE: `BODY[]` sets `\Seen` server-side as a side effect, so
        // `--seen` costs no extra round trip where `BODY.PEEK[]` does not.
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

    /// Appends a message and returns its UID, from the UIDPLUS
    /// `APPENDUID` or, failing that, a UID `SEARCH` on its `Message-ID`.
    ///
    /// RFC 4315 §3 makes `APPENDUID` a SHOULD, so an acknowledged append
    /// whose UID cannot be recovered is still a success, with no UID.
    pub fn add_message(
        &mut self,
        mailbox: &str,
        flags: &[Flag],
        raw: Vec<u8>,
    ) -> Result<Option<String>> {
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
            return Ok(Some(uid.to_string()));
        }

        // NOTE: without `APPENDUID`, which a server may omit even with
        // UIDPLUS, the UID is recovered by searching the message's own
        // `Message-ID`, which it therefore has to carry.
        let message_id = MessageParser::default()
            .parse_headers(&raw)
            .and_then(|parsed| parsed.message_id().map(str::to_string))
            .filter(|id| !id.is_empty());
        let Some(message_id) = message_id else {
            debug!("no APPENDUID and no Message-ID to search, appended UID unknown");
            return Ok(None);
        };

        self.select(mbox, ImapMailboxSelectOptions::default())?;

        let field =
            AString::try_from("Message-ID").map_err(|_| anyhow!("Invalid IMAP search header"))?;
        let value = AString::try_from(message_id.clone())
            .map_err(|_| anyhow!("Invalid IMAP search Message-ID value"))?;
        let criteria = Vec1::from(SearchKey::Header(field, value));
        let uids = self.search(criteria, ImapMessageSearchOptions { uid: true })?;

        let uid = uids.into_iter().max().map(|uid| uid.to_string());
        if uid.is_none() {
            debug!("no APPENDUID and no match for {message_id}, appended UID unknown");
        }

        Ok(uid)
    }

    /// Copies a UID set between two mailboxes.
    pub fn copy_messages(&mut self, from: &str, to: &str, ids: &[&str]) -> Result<usize> {
        let source = parse_mailbox(from)?;
        let target = parse_mailbox(to)?;
        let sequence_set = parse_uids(ids)?;

        self.select(source, ImapMailboxSelectOptions::default())?;
        let copy_uid = self.copy(
            sequence_set.clone(),
            target,
            ImapMessageCopyOptions { uid: true },
        )?;

        Ok(self.copied_uids(copy_uid, sequence_set)?.len())
    }

    /// Moves a UID set between two mailboxes.
    ///
    /// With RFC 6851 MOVE this is a single `UID MOVE`. Without it, it is
    /// the sequence RFC 6851 §1 describes for such servers: `UID COPY`,
    /// then `\Deleted` and, when RFC 4315 UIDPLUS is advertised, a
    /// `UID EXPUNGE` of the source UIDs really copied (see
    /// [`Self::copied_uids`]). Without UIDPLUS the messages stay flagged
    /// in the source, as `delete_messages` already does in the trash. A
    /// plain `EXPUNGE` would also remove every other `\Deleted` message.
    pub fn move_messages(&mut self, from: &str, to: &str, ids: &[&str]) -> Result<usize> {
        let source = parse_mailbox(from)?;
        let target = parse_mailbox(to)?;
        let sequence_set = parse_uids(ids)?;

        self.select(source, ImapMailboxSelectOptions::default())?;

        if self.supports_move() {
            // NOTE: `UID MOVE` takes the messages out of the source, so
            // without a readable `COPYUID` nothing is left there to tell
            // a moved message from one that never existed. The UIDs
            // present are searched beforehand, and only they are moved.
            let present = self.present_uids(sequence_set)?;
            if present.is_empty() {
                return Ok(0);
            }
            let copy_uid = self.r#move(
                uid_sequence_set(present.clone())?,
                target,
                ImapMessageMoveOptions { uid: true },
            )?;
            return Ok(match copy_uid {
                Some((_, source_uids, _)) => source_uids.len(),
                None => present.len(),
            });
        }

        let copy_uid = self.copy(
            sequence_set.clone(),
            target,
            ImapMessageCopyOptions { uid: true },
        )?;
        let copied = self.copied_uids(copy_uid, sequence_set)?;
        if copied.is_empty() {
            return Ok(0);
        }

        let moved = copied.len();
        let to_remove = uid_sequence_set(copied)?;

        self.store(
            to_remove.clone(),
            StoreType::Add,
            vec![ImapFlag::Deleted],
            ImapMessageStoreOptions { uid: true },
        )?;

        if self.supports_uidplus() {
            self.uid_expunge(to_remove)?;
        } else {
            debug!(
                "server lacks UIDPLUS, skipping UID EXPUNGE; \
                 messages stay flagged \\Deleted in the source"
            );
        }

        Ok(moved)
    }

    /// Permanently deletes a UID set from the trash, returning whether
    /// the messages were really removed.
    ///
    /// They are flagged `\Deleted`, then `UID EXPUNGE`d one by one when
    /// the server advertises RFC 4315 UIDPLUS, which leaves every other
    /// `\Deleted` message alone. Without it they are only flagged.
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

    /// The source UIDs a `UID COPY` of `requested` really copied, the
    /// source still selected.
    ///
    /// A RFC 4315 `COPYUID` lists exactly those, so it answers alone,
    /// including Mailo's, whose forbidden UIDVALIDITY 0 io-imap still
    /// reads ([#781]). Without one the tagged `OK` says nothing of what
    /// matched: a server without UIDPLUS never sends one, a UIDPLUS
    /// server sends none when nothing matched (an `OK` all the same,
    /// RFC 9051 §6.4.9), and one malformed beyond that deviation is
    /// reported as missing. `COPY` leaves the source untouched, so a
    /// `UID SEARCH` of the requested UIDs still there finds the ones
    /// copied.
    ///
    /// [#781]: https://github.com/pimalaya/himalaya/issues/781
    fn copied_uids(&mut self, copy_uid: ImapCopyUid, requested: SequenceSet) -> Result<Vec<u32>> {
        match copy_uid {
            Some((_, source_uids, _)) => Ok(source_uids),
            None => self.present_uids(requested),
        }
    }

    /// The UIDs of `requested` present in the selected mailbox, by a
    /// `UID SEARCH UID`, which leaves the absent ones out.
    fn present_uids(&mut self, requested: SequenceSet) -> Result<Vec<u32>> {
        let criteria = Vec1::from(SearchKey::Uid(requested));
        let uids = self.search(criteria, ImapMessageSearchOptions { uid: true })?;
        let mut uids: Vec<u32> = uids.into_iter().map(NonZeroU32::get).collect();
        uids.sort_unstable();
        uids.dedup();
        Ok(uids)
    }
}

/// One `LIST` row: a mailbox, its delimiter and its attributes.
type ListRow = (
    ImapMailbox<'static>,
    Option<io_imap::types::core::QuotedChar>,
    Vec<FlagNameAttribute<'static>>,
);

/// Drops `\Noselect` containers (RFC 3501 section 6.3.8): they cannot hold
/// messages and would error out on any later shared-API op.
fn is_selectable(row: &ListRow) -> bool {
    !row.2.contains(&FlagNameAttribute::Noselect)
}

/// Converts one `LIST` row into the shared [`Mailbox`] shape.
fn mailbox_from(row: ListRow) -> Mailbox {
    let role = matches!(row.0, ImapMailbox::Inbox).then_some(MailboxRole::Inbox);
    let name = match row.0 {
        ImapMailbox::Inbox => "Inbox".to_string(),
        ImapMailbox::Other(other) => String::from_utf8_lossy(other.inner().as_ref()).into_owned(),
    };

    Mailbox {
        id: name.clone(),
        name,
        role,
        total: None,
        unread: None,
    }
}

/// Folds a `STATUS` response into the matching mailbox row.
fn apply_status(mailbox: &mut Mailbox, items: Vec<StatusDataItem>) {
    for item in items {
        match item {
            StatusDataItem::Messages(n) => mailbox.total = Some(u64::from(n)),
            StatusDataItem::Unseen(n) => mailbox.unread = Some(u64::from(n)),
            _ => {}
        }
    }
}

/// The `FETCH` item names a listing asks for, `BODYSTRUCTURE` included
/// only when the attachment column is drawn.
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

/// The sequence set of one page, `None` when the window is empty. Page 1
/// is the most recent one.
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

/// Folds one `FETCH` row into a shared [`Envelope`].
fn envelope_from(seq: u32, items: Vec<MessageDataItem<'static>>) -> Envelope {
    let mut id = String::new();
    let mut message_id: Option<String> = None;
    let mut in_reply_to = Vec::new();
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
                // NOTE: the ninth ENVELOPE element (RFC 3501 section
                // 7.4.2), so a reply's parent costs no extra round trip.
                if let Some(m) = env.in_reply_to.into_option() {
                    in_reply_to = parse_message_ids(&bytes_to_string(m.as_ref()));
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
        in_reply_to,
        flags,
        subject,
        from,
        to,
        date,
        size,
        has_attachment,
    }
}

/// Converts a fetched flag into a shared [`Flag`], dropping the
/// non-flag fetch items.
fn flag_from_fetch(fetch: FlagFetch<'_>) -> Option<Flag> {
    let FlagFetch::Flag(flag) = fetch else {
        return None;
    };
    Some(Flag::from_raw(flag.to_string()))
}

/// Converts an ENVELOPE address into a shared [`Address`].
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

/// Whether a body structure holds a part disposed as an attachment.
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

/// Maps a shared [`Flag`] onto its IMAP wire counterpart.
///
/// An IANA flag becomes the matching system flag, and a custom keyword a
/// Keyword atom, sanitised when it is not atom-safe.
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

/// Replaces every non-atom-safe byte with `_`, so a keyword carrying
/// spaces, controls or brackets survives a `STORE`.
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
    let uids = ids
        .iter()
        .map(|s| s.parse().map_err(|_| anyhow!("Invalid message UID `{s}`")))
        .collect::<Result<_>>()?;

    uid_sequence_set(uids)
}

/// Builds an IMAP [`SequenceSet`] from numeric UIDs.
fn uid_sequence_set(uids: Vec<u32>) -> Result<SequenceSet> {
    if uids.is_empty() {
        bail!("Empty UID set");
    }

    let uids: Vec<NonZeroU32> = uids
        .into_iter()
        .map(|uid| NonZeroU32::new(uid).ok_or_else(|| anyhow!("Invalid message UID `{uid}`")))
        .collect::<Result<_>>()?;

    SequenceSet::try_from(uids).map_err(|_| anyhow!("Invalid UID set"))
}

/// Parses an RFC 2822 date, retrying without its day-of-week token.
fn parse_rfc2822_date(raw: &str) -> Option<DateTime<FixedOffset>> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    // NOTE: chrono rejects the whole timestamp when the day-of-week
    // disagrees with the date, which a surprising number of senders get
    // wrong. The token is redundant, so the retry drops it.
    DateTime::parse_from_rfc2822(trimmed)
        .or_else(|_| DateTime::parse_from_rfc2822(strip_weekday(trimmed)))
        .ok()
}

/// Drops the leading day-of-week token of an RFC 2822 date, leaving the
/// unambiguous remainder chrono parses without the weekday check.
fn strip_weekday(date: &str) -> &str {
    match date.split_once(", ") {
        Some((dow, rest)) if dow.len() == 3 && dow.bytes().all(|b| b.is_ascii_alphabetic()) => rest,
        _ => date,
    }
}

/// Renders bytes as UTF-8, falling back to latin-1 when they are not.
fn bytes_to_string(bytes: &[u8]) -> String {
    from_utf8(bytes).map(str::to_string).unwrap_or_else(|_| {
        let mut out = String::with_capacity(bytes.len());
        for b in bytes {
            out.push(*b as char);
        }
        out
    })
}

/// Decodes the RFC 2047 encoded words of an ENVELOPE string, falling back
/// to [`bytes_to_string`] on malformed input.
fn decode_mime_bytes(bytes: &[u8]) -> String {
    let decoder = Decoder::new().too_long_encoded_word_strategy(RecoverStrategy::Decode);
    decoder
        .decode(bytes)
        .unwrap_or_else(|_| bytes_to_string(bytes))
}

/// The `SEARCH` keys of a shared filter, `ALL` when there is none.
fn search_keys(filter: Option<&SearchEmailsFilterQuery>) -> Result<Vec1<SearchKey<'static>>> {
    let key = match filter {
        None => SearchKey::All,
        Some(filter) => convert_filter(filter)?,
    };
    Ok(Vec1::from(key))
}

/// The `SORT` criteria of a shared sort, `REVERSE DATE` when there is
/// none.
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

/// Translates one shared filter node into an IMAP search key.
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

        // NOTE: the shared clause is strictly after, where SENTSINCE is on
        // or after, so the day is bumped.
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

/// Translates one shared sorter into an IMAP sort criterion.
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

/// Wraps a search pattern in an IMAP astring.
fn astring(pattern: &str) -> Result<AString<'static>> {
    AString::try_from(String::from(pattern))
        .map_err(|_| anyhow!("Invalid IMAP search pattern `{pattern}`"))
}

/// Converts a date into the IMAP wire date a search key takes.
fn imap_date(date: chrono::NaiveDate) -> Result<ImapNaiveDate> {
    ImapNaiveDate::try_from(date).map_err(|_| anyhow!("Invalid IMAP date `{date}`"))
}

/// Slices one page out of the sorted UIDs, keeping their order.
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

/// Reorders a `FETCH` response into the requested UID order, dropping the
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

    /// Copy and move counts against a scripted IMAP server.
    mod copy_and_move {
        use std::{
            io::{BufRead, BufReader, Write},
            net::TcpListener,
            thread::{self, JoinHandle},
        };

        use crate::{config::ImapConfig, imap::client::ImapClient};

        /// The `COPYUID` code a scripted server answers for the source
        /// UIDs that matched, if any.
        type CopyUid = fn(&[u32]) -> Option<String>;

        /// A valid `COPYUID` for the matched UIDs, none when nothing
        /// matched, as RFC 4315 has it.
        fn valid(matched: &[u32]) -> Option<String> {
            (!matched.is_empty()).then(|| copyuid(7, matched))
        }

        /// Mailo's `COPYUID` with the forbidden UIDVALIDITY 0 ([#781]).
        ///
        /// [#781]: https://github.com/pimalaya/himalaya/issues/781
        fn uidvalidity_zero(matched: &[u32]) -> Option<String> {
            (!matched.is_empty()).then(|| copyuid(0, matched))
        }

        /// A `COPYUID` malformed beyond that deviation (a UID 0), which
        /// io-imap leaves unreadable.
        fn garbage(matched: &[u32]) -> Option<String> {
            (!matched.is_empty()).then(|| String::from("COPYUID 0 0 100"))
        }

        /// No `COPYUID` at all, as a server without UIDPLUS answers.
        fn none(_: &[u32]) -> Option<String> {
            None
        }

        fn copyuid(uid_validity: u32, matched: &[u32]) -> String {
            let set = |uids: Vec<u32>| {
                uids.iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            };
            let destination = (100..).take(matched.len()).collect();
            format!(
                "COPYUID {uid_validity} {} {}",
                set(matched.to_vec()),
                set(destination)
            )
        }

        fn parse_set(set: &str) -> Vec<u32> {
            set.split(',')
                .flat_map(|part| match part.split_once(':') {
                    Some((a, b)) => (a.parse().unwrap()..=b.parse().unwrap()).collect(),
                    None => vec![part.parse().unwrap()],
                })
                .collect()
        }

        /// Serves one session whose INBOX holds `existing`, returning the
        /// address and the commands it received, tags stripped.
        fn serve(
            capabilities: &'static str,
            mut existing: Vec<u32>,
            copy_uid: CopyUid,
        ) -> (String, JoinHandle<Vec<String>>) {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = listener.local_addr().unwrap();
            let handle = thread::spawn(move || {
                let (stream, _) = listener.accept().unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut out = stream;
                let mut commands = Vec::new();
                write!(out, "* PREAUTH [CAPABILITY {capabilities}] ready\r\n").unwrap();

                let mut line = String::new();
                while reader.read_line(&mut line).unwrap() > 0 {
                    let (tag, command) = line.trim_end().split_once(' ').unwrap();
                    let (tag, command) = (tag.to_string(), command.to_string());
                    line.clear();
                    commands.push(command.clone());
                    let words: Vec<&str> = command.split(' ').collect();
                    let reply = match words.as_slice() {
                        ["SELECT", ..] => format!(
                            "* {} EXISTS\r\n* OK [UIDVALIDITY 7] ok\r\n\
                             {tag} OK [READ-WRITE] done\r\n",
                            existing.len()
                        ),
                        ["UID", "SEARCH", "UID", set] => {
                            let found: Vec<String> = parse_set(set)
                                .into_iter()
                                .filter(|uid| existing.contains(uid))
                                .map(|uid| uid.to_string())
                                .collect();
                            let found = found.iter().map(|uid| format!(" {uid}"));
                            let found: String = found.collect();
                            format!("* SEARCH{found}\r\n{tag} OK done\r\n")
                        }
                        ["UID", verb @ ("COPY" | "MOVE"), set, _] => {
                            let matched: Vec<u32> = parse_set(set)
                                .into_iter()
                                .filter(|uid| existing.contains(uid))
                                .collect();
                            if *verb == "MOVE" {
                                existing.retain(|uid| !matched.contains(uid));
                            }
                            match copy_uid(&matched) {
                                Some(code) => format!("{tag} OK [{code}] done\r\n"),
                                None => format!("{tag} OK done\r\n"),
                            }
                        }
                        ["UID", "EXPUNGE", set] => {
                            let set = parse_set(set);
                            existing.retain(|uid| !set.contains(uid));
                            format!("{tag} OK done\r\n")
                        }
                        ["LOGOUT"] => {
                            write!(out, "* BYE bye\r\n{tag} OK done\r\n").unwrap();
                            break;
                        }
                        _ => format!("{tag} OK done\r\n"),
                    };
                    out.write_all(reply.as_bytes()).unwrap();
                }

                commands
            });

            (format!("imap://{addr}"), handle)
        }

        fn client(server: &str) -> ImapClient {
            let config: ImapConfig = toml::from_str(&format!("server = \"{server}\"")).unwrap();
            ImapClient::new(config).unwrap()
        }

        /// Runs `op` against a scripted server, returning its count and
        /// the commands the server received.
        fn run(
            capabilities: &'static str,
            existing: &[u32],
            copy_uid: CopyUid,
            op: impl FnOnce(&mut ImapClient) -> anyhow::Result<usize>,
        ) -> (usize, Vec<String>) {
            let (server, handle) = serve(capabilities, existing.to_vec(), copy_uid);
            let mut client = client(&server);
            let count = op(&mut client).unwrap();
            drop(client);
            (count, handle.join().unwrap())
        }

        fn has(commands: &[String], prefix: &str) -> bool {
            commands.iter().any(|command| command.starts_with(prefix))
        }

        const UIDPLUS: &str = "IMAP4rev1 UIDPLUS";
        const PLAIN: &str = "IMAP4rev1";
        const MOVE: &str = "IMAP4rev1 UIDPLUS MOVE";

        fn mv(
            ids: &'static [&'static str],
        ) -> impl FnOnce(&mut ImapClient) -> anyhow::Result<usize> {
            move |client| client.move_messages("INBOX", "Archive", ids)
        }

        #[test]
        fn a_valid_copyuid_answers_the_copy_fallback_move() {
            let (count, commands) = run(UIDPLUS, &[1], valid, mv(&["1", "999"]));
            assert_eq!(count, 1);
            assert!(!has(&commands, "UID SEARCH"), "{commands:?}");
            assert!(has(&commands, "UID STORE 1 +FLAGS"), "{commands:?}");
            assert!(has(&commands, "UID EXPUNGE 1"), "{commands:?}");
        }

        #[test]
        fn a_uidvalidity_zero_copyuid_answers_the_copy_fallback_move() {
            let (count, commands) = run(UIDPLUS, &[1], uidvalidity_zero, mv(&["1", "999"]));
            assert_eq!(count, 1);
            assert!(!has(&commands, "UID SEARCH"), "{commands:?}");
            assert!(has(&commands, "UID STORE 1 +FLAGS"), "{commands:?}");
            assert!(has(&commands, "UID EXPUNGE 1"), "{commands:?}");
        }

        #[test]
        fn an_unreadable_copyuid_still_completes_the_move() {
            let (count, commands) = run(UIDPLUS, &[1], garbage, mv(&["1", "999"]));
            assert_eq!(count, 1);
            assert!(has(&commands, "UID SEARCH UID 1,999"), "{commands:?}");
            assert!(has(&commands, "UID STORE 1 +FLAGS"), "{commands:?}");
            assert!(has(&commands, "UID EXPUNGE 1"), "{commands:?}");
        }

        #[test]
        fn no_copyuid_from_a_uidplus_server_is_a_message_not_found() {
            let (count, commands) = run(UIDPLUS, &[1], valid, mv(&["999"]));
            assert_eq!(count, 0);
            assert!(!has(&commands, "UID STORE"), "{commands:?}");
            assert!(!has(&commands, "UID EXPUNGE"), "{commands:?}");
        }

        #[test]
        fn without_uidplus_the_source_tells_what_was_copied() {
            let (count, commands) = run(PLAIN, &[1], none, mv(&["1"]));
            assert_eq!(count, 1);
            assert!(has(&commands, "UID STORE 1 +FLAGS"), "{commands:?}");
            assert!(!has(&commands, "UID EXPUNGE"), "{commands:?}");

            let (count, commands) = run(PLAIN, &[1], none, mv(&["999"]));
            assert_eq!(count, 0);
            assert!(!has(&commands, "UID STORE"), "{commands:?}");
        }

        #[test]
        fn a_move_without_copyuid_counts_the_messages_present() {
            let (count, commands) = run(MOVE, &[1], none, mv(&["1", "999"]));
            assert_eq!(count, 1);
            assert!(has(&commands, "UID MOVE 1 "), "{commands:?}");

            let (count, commands) = run(MOVE, &[1], valid, mv(&["999"]));
            assert_eq!(count, 0);
            assert!(!has(&commands, "UID MOVE"), "{commands:?}");
        }

        #[test]
        fn a_copy_counts_what_was_copied() {
            let copy = |ids: &'static [&'static str]| {
                move |client: &mut ImapClient| client.copy_messages("INBOX", "Archive", ids)
            };
            assert_eq!(run(UIDPLUS, &[1, 2], valid, copy(&["1", "2"])).0, 2);
            let (count, commands) = run(UIDPLUS, &[1], uidvalidity_zero, copy(&["1"]));
            assert_eq!(count, 1);
            assert!(!has(&commands, "UID SEARCH"), "{commands:?}");
            let (count, commands) = run(UIDPLUS, &[1], garbage, copy(&["1", "999"]));
            assert_eq!(count, 1);
            assert!(has(&commands, "UID SEARCH UID 1,999"), "{commands:?}");
            assert_eq!(run(UIDPLUS, &[1], valid, copy(&["999"])).0, 0);
            assert_eq!(run(PLAIN, &[1], none, copy(&["1", "999"])).0, 1);
        }
    }
}
