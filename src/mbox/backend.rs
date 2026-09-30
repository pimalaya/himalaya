//! # mbox backend
//!
//! The mbox adapter of the shared cross-protocol client, glue over the
//! io-mbox client [`MboxClient`] wraps.
//!
//! A mailbox is one mbox file and a message is addressed by its io-mbox
//! content id, which a flag write, by Himalaya or any other client, does
//! not change. Every operation first syncs the file's index through the
//! [`crate::mbox::cache`], and a listing parses only the messages it has
//! not seen before.

use std::collections::BTreeMap;

use anyhow::{Result, anyhow};
use chrono::DateTime;
use io_mbox::{
    client::MboxClientError,
    entry::{
        MboxEntry,
        append::{MboxEntryAppend, MboxEntryAppendItem, MboxEntryAppendOptions},
        copy::{MboxEntryCopy, MboxEntryCopyOptions},
        get::{MboxEntryGet, MboxEntryGetError, MboxEntryGetOptions},
        r#move::{MboxEntryMove, MboxEntryMoveOptions},
        update::{MboxEntryUpdate, MboxEntryUpdateEdit, MboxEntryUpdateOptions},
    },
    flag::{MboxFlag, MboxFlags},
    index::MboxIndex,
    path::MboxFsPath,
    store::INBOX,
};
use mail_parser::{Address as MailParserAddress, HeaderValue, MessageParser};

use crate::{
    email::{
        address::Address,
        envelope::{Envelope, normalize_message_id, parse_message_ids},
        flag::{Flag, FlagOp, IanaFlag},
        mailbox::{Mailbox, MailboxRole},
        search::{eval, query::SearchEmailsQuery},
    },
    mbox::{cache::MboxCache, client::MboxClient},
};

impl MboxClient {
    /// Lists the mbox files of the store, the spool as `INBOX`.
    ///
    /// `with_counts` syncs the index of every file, which scans a file the
    /// cache does not know yet.
    pub fn list_mailboxes(&self, with_counts: bool) -> Result<Vec<Mailbox>> {
        let mut mailboxes = Vec::new();

        for mbox in self.list_mboxes()? {
            let name = mbox.name.to_string();
            let role = (self.store.inbox.is_some() && name == INBOX).then_some(MailboxRole::Inbox);

            let (total, unread) = if with_counts {
                let index = self.index(&mbox.path, false)?.0;
                let total = index.messages().count() as u64;
                let unread = index
                    .messages()
                    .filter(|entry| !entry.flags.contains(&MboxFlag::Seen))
                    .count() as u64;
                (Some(total), Some(unread))
            } else {
                (None, None)
            };

            mailboxes.push(Mailbox {
                id: name.clone(),
                name,
                role,
                total,
                unread,
            });
        }

        Ok(mailboxes)
    }

    /// Lists envelopes from `mailbox`, sorted by `Date:` descending then
    /// paginated.
    pub fn list_envelopes(
        &self,
        mailbox: &str,
        page: Option<u32>,
        page_size: Option<u32>,
        _with_attachment: bool,
    ) -> Result<Vec<Envelope>> {
        let path = self.resolve_mbox(mailbox);
        let mut envelopes: Vec<Envelope> = self
            .envelopes(&path, false)?
            .into_iter()
            .map(|(envelope, _)| envelope)
            .collect();
        envelopes.sort_by_key(|envelope| std::cmp::Reverse(envelope.date));

        Ok(paginate(envelopes, page, page_size))
    }

    /// Searches envelopes in `mailbox`: reads every message, then applies
    /// the shared filter, sort and pagination client-side.
    pub fn search_envelopes(
        &self,
        mailbox: &str,
        query: Option<&SearchEmailsQuery>,
        page: Option<u32>,
        page_size: Option<u32>,
        _with_attachment: bool,
    ) -> Result<Vec<Envelope>> {
        let path = self.resolve_mbox(mailbox);
        let filter = query.and_then(|q| q.filter.as_ref());

        let mut hits: Vec<Envelope> = Vec::new();
        for (envelope, contents) in self.envelopes(&path, filter.is_some())? {
            let keep = match (filter, contents) {
                (Some(filter), Some(contents)) => {
                    eval::matches_filter(&envelope, &contents, filter)
                }
                _ => true,
            };
            if keep {
                hits.push(envelope);
            }
        }

        eval::sort_envelopes(&mut hits, query.and_then(|q| q.sort.as_deref()));
        Ok(paginate(hits, page, page_size))
    }

    /// Adds, sets or removes `flags` on messages of `mailbox`, rewriting
    /// the file once for the whole set.
    ///
    /// A set keeps the `O` (old) status, which no shared flag names, so
    /// replacing the flags does not make a read message new again, and a
    /// seen message is marked old too.
    pub fn store_flags(
        &self,
        mailbox: &str,
        ids: &[&str],
        flags: &[Flag],
        op: FlagOp,
    ) -> Result<()> {
        let path = self.resolve_mbox(mailbox);
        let flags = flags_to_mbox(flags);

        self.edit_flags(&path, ids, |current| {
            let mut next = current.clone();
            match op {
                FlagOp::Add => next.extend(flags.iter().cloned()),
                FlagOp::Remove => {
                    for flag in flags.iter() {
                        next.remove(flag);
                    }
                }
                FlagOp::Set => {
                    next = flags.clone();
                    if current.contains(&MboxFlag::Old) {
                        next.insert(MboxFlag::Old);
                    }
                }
            }
            read_is_old(&mut next);
            next
        })
    }

    /// Rewrites the flags of messages of the mbox at `path` to what `edit`
    /// makes of their current ones, in one locked rewrite of the file.
    pub fn edit_flags(
        &self,
        path: &MboxFsPath,
        ids: &[&str],
        edit: impl Fn(&MboxFlags) -> MboxFlags,
    ) -> Result<()> {
        let (index, mut cache) = self.index(path, false)?;

        let mut edits = BTreeMap::new();
        for id in ids {
            let entry = find(&index, id, path)?;
            let flags = edit(&entry.flags);
            edits.insert(id.to_string(), MboxEntryUpdateEdit::Flags(flags));
        }

        self.update(path, edits, index, &mut cache)
    }

    /// Reads one message's raw RFC 5322 bytes, unquoted, from `mailbox`.
    ///
    /// An index another client made stale is rebuilt and the read tried
    /// once more.
    pub fn get_message(&self, mailbox: &str, id: &str, seen: bool) -> Result<Vec<u8>> {
        let path = self.resolve_mbox(mailbox);
        let (index, _) = self.index(&path, false)?;

        let contents = match self.read(&path, find(&index, id, &path)?) {
            Err(MboxClientError::EntryGet(MboxEntryGetError::Stale(_))) => {
                let (index, _) = self.index(&path, true)?;
                self.read(&path, find(&index, id, &path)?)?
            }
            result => result?,
        };

        if seen {
            let seen = Flag::from_iana(IanaFlag::Seen);
            self.store_flags(mailbox, &[id], &[seen], FlagOp::Add)?;
        }

        Ok(contents)
    }

    /// Appends `raw` to `mailbox` with `flags`, returning its id.
    pub fn add_message(&self, mailbox: &str, flags: &[Flag], raw: Vec<u8>) -> Result<String> {
        let path = self.resolve_mbox(mailbox);
        let mut flags = flags_to_mbox(flags);
        read_is_old(&mut flags);
        self.append(&path, flags, raw)
    }

    /// Appends `raw` to the mbox at `path` with `flags` under lock,
    /// returning its id.
    pub fn append(&self, path: &MboxFsPath, flags: MboxFlags, raw: Vec<u8>) -> Result<String> {
        let item = MboxEntryAppendItem {
            contents: raw,
            flags,
            ..Default::default()
        };
        let opts = MboxEntryAppendOptions {
            format: self.format,
            lock: self.lock.clone(),
            scanner: self.scanner.clone(),
        };
        let appended = self.run(MboxEntryAppend::new(path.clone(), vec![item], opts))?;
        let offset = appended.first().map(|entry| entry.offset);

        // NOTE: the append knows the content id, not how many copies of
        // the message the file already holds, which the numbered id needs.
        let (index, _) = self.index(path, false)?;
        index
            .entries
            .iter()
            .find(|entry| Some(entry.offset) == offset)
            .map(|entry| entry.id.clone())
            .ok_or_else(|| anyhow!("Cannot find the message appended to {path}"))
    }

    /// Copies every id from `from` to the end of `to`.
    pub fn copy_messages(&self, from: &str, to: &str, ids: &[&str]) -> Result<usize> {
        let source = self.resolve_mbox(from);
        let target = self.resolve_mbox(to);
        let (index, _) = self.index(&source, false)?;
        let entries = ids
            .iter()
            .map(|id| find(&index, id, &source).cloned())
            .collect::<Result<Vec<_>>>()?;

        let opts = MboxEntryCopyOptions {
            format: self.format,
            lock: self.lock.clone(),
            scanner: self.scanner.clone(),
            chunk_size: self.chunk_size,
        };
        let copied = self.run(MboxEntryCopy::new(source, target, entries, opts))?;
        Ok(copied.len())
    }

    /// Moves every id from `from` to the end of `to`: a copy, then a
    /// removal from the source.
    pub fn move_messages(&self, from: &str, to: &str, ids: &[&str]) -> Result<usize> {
        let source = self.resolve_mbox(from);
        let target = self.resolve_mbox(to);
        let (index, mut cache) = self.index(&source, false)?;
        let entries = ids
            .iter()
            .map(|id| find(&index, id, &source).cloned())
            .collect::<Result<Vec<_>>>()?;

        let opts = MboxEntryMoveOptions {
            index: Some(index),
            format: self.format,
            lock: self.lock.clone(),
            scanner: self.scanner.clone(),
            chunk_size: self.chunk_size,
        };
        let output = self.run(MboxEntryMove::new(source.clone(), target, entries, opts))?;
        let moved = output.entries.len();

        cache.index = Some(output.index);
        cache.save(self.cache_dir.as_deref(), &source);
        Ok(moved)
    }

    /// Permanently removes `ids` from `mailbox`.
    pub fn delete_messages(&self, mailbox: &str, ids: &[&str]) -> Result<()> {
        let path = self.resolve_mbox(mailbox);
        let (index, mut cache) = self.index(&path, false)?;

        let mut edits = BTreeMap::new();
        for id in ids {
            find(&index, id, &path)?;
            edits.insert(id.to_string(), MboxEntryUpdateEdit::Remove);
        }

        self.update(&path, edits, index, &mut cache)
    }

    /// The mailbox marked with `role`: the spool is the inbox, when one is
    /// configured.
    pub fn role_mailbox_id(&self, role: &MailboxRole) -> Option<String> {
        (*role == MailboxRole::Inbox && self.store.inbox.is_some()).then(|| INBOX.to_string())
    }

    /// Syncs the index of `path`, returning it beside the rest of the
    /// cache.
    fn index(&self, path: &MboxFsPath, rebuild: bool) -> Result<(MboxIndex, MboxCache)> {
        let mut cache = self.sync(path, rebuild)?;
        let index = cache.index.take().unwrap_or_default();
        Ok((index, cache))
    }

    /// Reads the message of `entry`, unquoted.
    fn read(&self, path: &MboxFsPath, entry: &MboxEntry) -> Result<Vec<u8>, MboxClientError> {
        let opts = MboxEntryGetOptions {
            format: self.format,
            scanner: self.scanner.clone(),
            chunk_size: self.chunk_size,
        };
        let message = self.run(MboxEntryGet::new(path.clone(), entry.clone(), opts))?;
        Ok(message.contents)
    }

    /// Applies `edits` to `path` under lock, and caches the rewritten
    /// index.
    fn update(
        &self,
        path: &MboxFsPath,
        edits: BTreeMap<String, MboxEntryUpdateEdit>,
        index: MboxIndex,
        cache: &mut MboxCache,
    ) -> Result<()> {
        let opts = MboxEntryUpdateOptions {
            index: Some(index),
            lock: self.lock.clone(),
            scanner: self.scanner.clone(),
            chunk_size: self.chunk_size,
        };
        let output = self.run(MboxEntryUpdate::new(path.clone(), edits, opts))?;

        cache.index = Some(output.index);
        cache.save(self.cache_dir.as_deref(), path);
        Ok(())
    }

    /// The envelope of every message of `path`, pseudo message aside, in
    /// file order, with its bytes when `contents` is set.
    ///
    /// Envelopes come from the cache when it has them, the flags always
    /// from the fresh index. A message the cache lacks is read and parsed
    /// once, then cached.
    fn envelopes(
        &self,
        path: &MboxFsPath,
        contents: bool,
    ) -> Result<Vec<(Envelope, Option<Vec<u8>>)>> {
        let (index, mut cache) = self.index(path, false)?;
        let mut envelopes = Vec::new();
        let mut parsed = false;

        for entry in index.messages() {
            let cached = cache.envelopes.get(&entry.id).cloned();
            let bytes = match (&cached, contents) {
                (Some(_), false) => None,
                _ => Some(self.read(path, entry)?),
            };

            let mut envelope = match cached {
                Some(envelope) => envelope,
                None => {
                    let envelope = envelope_from(bytes.as_deref().unwrap_or_default());
                    cache.envelopes.insert(entry.id.clone(), envelope.clone());
                    parsed = true;
                    envelope
                }
            };

            envelope.id = entry.id.clone();
            envelope.flags = entry.flags.iter().filter_map(flag_from_mbox).collect();
            envelopes.push((envelope, bytes));
        }

        if parsed {
            cache.index = Some(index);
            cache.save(self.cache_dir.as_deref(), path);
        }

        Ok(envelopes)
    }
}

/// Finds the entry `id` names in `index`.
fn find<'a>(index: &'a MboxIndex, id: &str, path: &MboxFsPath) -> Result<&'a MboxEntry> {
    index
        .get(id)
        .ok_or_else(|| anyhow!("Message `{id}` not found in mbox {path}"))
}

/// Parses a message into a shared [`Envelope`], its id and flags left
/// empty for the caller to fill from the index.
fn envelope_from(contents: &[u8]) -> Envelope {
    let parsed = MessageParser::new().parse(contents);

    let subject = parsed
        .as_ref()
        .and_then(|m| m.subject())
        .unwrap_or_default()
        .to_string();

    let from = parsed
        .as_ref()
        .and_then(|m| m.from())
        .map(addresses_from)
        .unwrap_or_default();

    let to = parsed
        .as_ref()
        .and_then(|m| m.to())
        .map(addresses_from)
        .unwrap_or_default();

    let date = parsed
        .as_ref()
        .and_then(|m| m.date())
        .and_then(|d| DateTime::parse_from_rfc3339(&d.to_rfc3339()).ok());

    let has_attachment = parsed.as_ref().map(|m| m.attachment_count() > 0);

    let message_id = parsed
        .as_ref()
        .and_then(|m| m.message_id())
        .and_then(normalize_message_id);

    let in_reply_to = parsed
        .as_ref()
        .map(|m| ids_from_header(m.in_reply_to()))
        .unwrap_or_default();

    Envelope {
        id: String::new(),
        message_id,
        in_reply_to,
        flags: Default::default(),
        subject,
        from,
        to,
        date,
        size: contents.len() as u64,
        has_attachment,
    }
}

/// Reads a mail-parser msg-id header into the bare ids it names.
fn ids_from_header(value: &HeaderValue<'_>) -> Vec<String> {
    match value {
        HeaderValue::TextList(ids) => ids
            .iter()
            .filter_map(|id| normalize_message_id(id))
            .collect(),
        HeaderValue::Text(id) => parse_message_ids(id),
        _ => Vec::new(),
    }
}

/// mail-parser address group to a shared [`Address`] list.
fn addresses_from(addrs: &MailParserAddress<'_>) -> Vec<Address> {
    addrs
        .clone()
        .into_list()
        .into_iter()
        .filter_map(|a| {
            let email = a.address?.into_owned();
            if email.is_empty() {
                return None;
            }
            let name = a.name.map(|s| s.into_owned());
            Some(Address { name, email })
        })
        .collect()
}

/// Maps a shared [`Flag`] to an [`MboxFlag`]; a flag with no `Status`
/// or `X-Status` letter goes to `X-Keywords`.
fn flag_to_mbox(flag: &Flag) -> MboxFlag {
    match flag.iana() {
        Some(IanaFlag::Seen) => MboxFlag::Seen,
        Some(IanaFlag::Answered) => MboxFlag::Answered,
        Some(IanaFlag::Flagged) => MboxFlag::Flagged,
        Some(IanaFlag::Draft) => MboxFlag::Draft,
        Some(IanaFlag::Deleted) => MboxFlag::Deleted,
        Some(_) | None => MboxFlag::Keyword(flag.raw().to_string()),
    }
}

/// Marks a seen message old too, as mutt writes `Status: RO`: GNU mail
/// still lists a message whose `Status` lacks the `O` as new.
fn read_is_old(flags: &mut MboxFlags) {
    if flags.contains(&MboxFlag::Seen) {
        flags.insert(MboxFlag::Old);
    }
}

/// Shared flag slice to [`MboxFlags`].
fn flags_to_mbox(flags: &[Flag]) -> MboxFlags {
    flags.iter().map(flag_to_mbox).collect()
}

/// Maps an [`MboxFlag`] to a shared [`Flag`], the inverse of
/// [`flag_to_mbox`]. `O` (old) has no shared counterpart.
fn flag_from_mbox(flag: &MboxFlag) -> Option<Flag> {
    let flag = match flag {
        MboxFlag::Seen => Flag::from_iana(IanaFlag::Seen),
        MboxFlag::Answered => Flag::from_iana(IanaFlag::Answered),
        MboxFlag::Flagged => Flag::from_iana(IanaFlag::Flagged),
        MboxFlag::Draft => Flag::from_iana(IanaFlag::Draft),
        MboxFlag::Deleted => Flag::from_iana(IanaFlag::Deleted),
        MboxFlag::Keyword(keyword) => Flag::from_raw(keyword),
        MboxFlag::Old => return None,
    };
    Some(flag)
}

/// 1-indexed in-memory pagination; `page_size = None` returns the full
/// slice; size 0 or a page past the end returns empty.
fn paginate<T>(items: Vec<T>, page: Option<u32>, page_size: Option<u32>) -> Vec<T> {
    let Some(size) = page_size else {
        return items;
    };
    if size == 0 {
        return Vec::new();
    }
    let page = page.unwrap_or(1).max(1);
    let skip = ((page - 1) as usize).saturating_mul(size as usize);
    if skip >= items.len() {
        return Vec::new();
    }
    items.into_iter().skip(skip).take(size as usize).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mbox_flags_round_trip_through_the_shared_flag() {
        let flags = [
            MboxFlag::Seen,
            MboxFlag::Answered,
            MboxFlag::Flagged,
            MboxFlag::Draft,
            MboxFlag::Deleted,
            MboxFlag::Keyword("NonJunk".into()),
            MboxFlag::Keyword("$Junk".into()),
        ];

        for flag in flags {
            let shared = flag_from_mbox(&flag).unwrap();
            assert_eq!(flag_to_mbox(&shared), flag);
        }

        assert_eq!(flag_from_mbox(&MboxFlag::Old), None);
    }

    #[test]
    fn a_seen_message_is_old() {
        let mut flags = MboxFlags::from_iter([MboxFlag::Seen]);
        read_is_old(&mut flags);
        assert!(flags.contains(&MboxFlag::Old));

        let mut flags = MboxFlags::from_iter([MboxFlag::Flagged]);
        read_is_old(&mut flags);
        assert!(!flags.contains(&MboxFlag::Old));
    }

    #[test]
    fn envelopes_come_from_the_headers() {
        let envelope = envelope_from(
            b"Subject: Hi\r\nFrom: A <a@x.org>\r\nMessage-ID: <1@x>\r\nIn-Reply-To: <0@x>\r\n\r\nbody",
        );

        assert_eq!(envelope.subject, "Hi");
        assert_eq!(envelope.from[0].email, "a@x.org");
        assert_eq!(envelope.message_id.as_deref(), Some("1@x"));
        assert_eq!(envelope.in_reply_to, ["0@x"]);
        assert_eq!(envelope.has_attachment, Some(false));
    }
}
