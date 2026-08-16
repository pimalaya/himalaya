//! m2dir adapter for the shared cross-protocol client.
//!
//! Thin glue over [`M2dirClient`], which wraps io_m2dir's high-level
//! client (`list_m2dirs`, `open_m2dir`, `list_entries`, `read_entry`,
//! `get`, `store`, `delete_entry`, `add_flags`/`remove_flags`/`set_flags`).
//! m2dir is content-addressed and has no native copy/move, so those are
//! get + store (+ delete). The conversion is lifted from the retired
//! io-email m2dir drivers.

use std::path::Path;

use anyhow::Result;
use chrono::DateTime;
use io_m2dir::{entry::M2dirEntry, flag::M2dirFlags, m2dir::M2dir};
use mail_parser::{
    Address as MailParserAddress, HeaderValue, Message as ParsedMessage, MessageParser,
};

use crate::{
    email::{
        address::Address,
        envelope::{Envelope, normalize_message_id, parse_message_ids},
        flag::{Flag, FlagOp, IanaFlag},
        mailbox::Mailbox,
        search::{eval, query::SearchEmailsQuery},
    },
    m2dir::client::M2dirClient,
};

impl M2dirClient {
    /// Resolves a shared-layer mailbox argument to an opened [`M2dir`].
    ///
    /// An absolute path — the `id` column of `mailbox list` — opens
    /// directly; a relative name (`Inbox`, `Archive`, `Projects/Work`)
    /// is resolved under the m2store root first, with the spec's
    /// percent-encoding, since io-m2dir's `open_m2dir` only accepts a
    /// filesystem path. This lets the shared commands address a folder
    /// by name or by id, matching the raw `m2dir` commands (which take a
    /// name) and the Maildir backend.
    fn resolve_m2dir(&self, mailbox: &str) -> Result<M2dir> {
        if Path::new(mailbox).is_absolute() {
            return Ok(self.open_m2dir(mailbox)?);
        }

        let store = self.open_store()?;
        let path = store.resolve_folder_path(mailbox)?;
        Ok(self.open_m2dir(path)?)
    }

    /// Lists every m2dir under the configured root, sorted by name.
    /// `with_counts` is ignored (m2dir does not surface counts cheaply).
    pub fn list_mailboxes(&self, _with_counts: bool) -> Result<Vec<Mailbox>> {
        let mut mailboxes: Vec<Mailbox> =
            self.list_m2dirs()?.into_iter().map(mailbox_from).collect();
        mailboxes.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(mailboxes)
    }

    /// Lists envelopes from `mailbox`, sorted by `Date:` descending then
    /// paginated.
    pub fn list_envelopes(
        &self,
        mailbox: &str,
        page: Option<u32>,
        page_size: Option<u32>,
        with_attachment: bool,
    ) -> Result<Vec<Envelope>> {
        let m2dir = self.resolve_m2dir(mailbox)?;
        let entries = self.list_entries(m2dir.clone())?;

        let mut envelopes = Vec::with_capacity(entries.len());
        for entry in &entries {
            let bytes = self.read_entry(entry)?;
            let Some(parsed) = MessageParser::default().parse(&bytes) else {
                continue;
            };
            let flags = self.read_flags(&m2dir, entry.id())?;
            let mut envelope = envelope_from(entry, &flags, &parsed);
            if with_attachment {
                envelope.has_attachment = Some(parsed.attachment_count() > 0);
            }
            envelopes.push(envelope);
        }
        envelopes.sort_by_key(|envelope| std::cmp::Reverse(envelope.date));

        Ok(paginate(envelopes, page, page_size))
    }

    /// Searches envelopes in `mailbox`: lists them, then applies the
    /// shared filter/sort/paginate client-side (body clauses reuse the
    /// already-read message bytes).
    pub fn search_envelopes(
        &self,
        mailbox: &str,
        query: Option<&SearchEmailsQuery>,
        page: Option<u32>,
        page_size: Option<u32>,
        _with_attachment: bool,
    ) -> Result<Vec<Envelope>> {
        let m2dir = self.resolve_m2dir(mailbox)?;
        let entries = self.list_entries(m2dir.clone())?;

        let filter = query.and_then(|q| q.filter.as_ref());
        let mut hits = Vec::new();
        for entry in &entries {
            let bytes = self.read_entry(entry)?;
            let Some(parsed) = MessageParser::default().parse(&bytes) else {
                continue;
            };
            let flags = self.read_flags(&m2dir, entry.id())?;
            let envelope = envelope_from(entry, &flags, &parsed);
            let keep = match filter {
                Some(filter) => eval::matches_filter(&envelope, &bytes, filter),
                None => true,
            };
            if keep {
                hits.push(envelope);
            }
        }

        eval::sort_envelopes(&mut hits, query.and_then(|q| q.sort.as_deref()));
        Ok(paginate(hits, page, page_size))
    }

    /// Adds, sets, or removes `flags` on an m2dir id set.
    pub fn store_flags(
        &self,
        mailbox: &str,
        ids: &[&str],
        flags: &[Flag],
        op: FlagOp,
    ) -> Result<()> {
        let m2dir = self.resolve_m2dir(mailbox)?;
        let m2dir_flags = flags_to_m2dir(flags);

        for id in ids {
            match op {
                FlagOp::Add => self.add_flags(&m2dir, *id, m2dir_flags.clone())?,
                FlagOp::Set => self.set_flags(&m2dir, *id, m2dir_flags.clone())?,
                FlagOp::Remove => self.remove_flags(&m2dir, *id, m2dir_flags.clone())?,
            }
        }

        Ok(())
    }

    /// Reads one message's raw RFC 5322 bytes from `mailbox`.
    pub fn get_message(&self, mailbox: &str, id: &str, seen: bool) -> Result<Vec<u8>> {
        let m2dir = self.resolve_m2dir(mailbox)?;
        let (_entry, bytes) = self.get(m2dir, id)?;

        // Reading the bytes never touches the sidecar; `--seen` adds the
        // `S` flag so the read stays non-mutating by default.
        if seen {
            let seen = Flag::from_iana(IanaFlag::Seen);
            self.store_flags(mailbox, &[id], &[seen], FlagOp::Add)?;
        }

        Ok(bytes)
    }

    /// Stores `raw` under `mailbox`, then writes `flags` to the sidecar.
    /// Returns the content-addressed id.
    pub fn add_message(&self, mailbox: &str, flags: &[Flag], raw: Vec<u8>) -> Result<String> {
        let m2dir = self.resolve_m2dir(mailbox)?;
        let entry = self.store(m2dir.clone(), raw)?;
        let id = entry.id().to_string();

        if !flags.is_empty() {
            self.set_flags(&m2dir, &id, flags_to_m2dir(flags))?;
        }

        Ok(id)
    }

    /// Copies every id from `from` to `to` (get + store; flags are not
    /// propagated, matching io-email).
    pub fn copy_messages(&self, from: &str, to: &str, ids: &[&str]) -> Result<usize> {
        let source = self.resolve_m2dir(from)?;
        let target = self.resolve_m2dir(to)?;

        for id in ids {
            let (_entry, bytes) = self.get(source.clone(), *id)?;
            self.store(target.clone(), bytes)?;
        }

        Ok(ids.len())
    }

    /// Moves every id from `from` to `to` (copy then delete the source).
    pub fn move_messages(&self, from: &str, to: &str, ids: &[&str]) -> Result<usize> {
        let source = self.resolve_m2dir(from)?;
        let target = self.resolve_m2dir(to)?;

        for id in ids {
            let (_entry, bytes) = self.get(source.clone(), *id)?;
            self.store(target.clone(), bytes)?;
            self.delete_entry(source.clone(), *id)?;
        }

        Ok(ids.len())
    }

    /// Permanently deletes `ids` from `mailbox` by unlinking their files.
    pub fn delete_messages(&self, mailbox: &str, ids: &[&str]) -> Result<()> {
        let m2dir = self.resolve_m2dir(mailbox)?;

        for id in ids {
            self.delete_entry(m2dir.clone(), *id)?;
        }

        Ok(())
    }
}

/// Converts one [`M2dir`] into the shared [`Mailbox`] shape: `id` is the
/// on-disk path, `name` is the last path segment.
fn mailbox_from(m2dir: M2dir) -> Mailbox {
    let path = m2dir.path();
    Mailbox {
        id: path.as_str().to_string(),
        name: path.file_name().unwrap_or("").to_string(),
        total: None,
        unread: None,
    }
}

/// Folds an entry + its meta flags + parsed message into an [`Envelope`].
fn envelope_from(entry: &M2dirEntry, meta: &M2dirFlags, parsed: &ParsedMessage<'_>) -> Envelope {
    let id = entry.id().to_string();
    let flags = meta.iter().map(flag_from_meta_line).collect();
    let subject = parsed.subject().unwrap_or_default().to_string();
    let from = parsed.from().map(addresses_from).unwrap_or_default();
    let to = parsed.to().map(addresses_from).unwrap_or_default();
    let date = parsed
        .date()
        .and_then(|d| DateTime::parse_from_rfc3339(&d.to_rfc3339()).ok());
    let size = parsed.raw_message().len() as u64;
    let message_id = parsed.message_id().and_then(normalize_message_id);
    let in_reply_to = match parsed.in_reply_to() {
        HeaderValue::TextList(ids) => ids
            .iter()
            .filter_map(|id| normalize_message_id(id))
            .collect(),
        HeaderValue::Text(id) => parse_message_ids(id),
        _ => Vec::new(),
    };

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
        has_attachment: None,
    }
}

/// One `.meta/<id>.flags` line to a shared [`Flag`]; whitespace trimmed.
fn flag_from_meta_line(line: &str) -> Flag {
    Flag::from_raw(line.trim())
}

/// Shared flag slice to [`M2dirFlags`], one canonical line per flag.
fn flags_to_m2dir(flags: &[Flag]) -> M2dirFlags {
    flags.iter().map(|flag| flag.raw().to_string()).collect()
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
