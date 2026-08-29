//! # Maildir backend
//!
//! The Maildir adapter of the shared cross-protocol client, glue over the
//! io-maildir client [`MaildirClient`] wraps.
//!
//! Each method takes and returns the CLI's own [`crate::email`] types, so
//! the work here is converting between those and io-maildir's.

use std::path::Path;

use anyhow::Result;
use chrono::DateTime;
use io_maildir::{
    entry::MaildirFullEntry,
    flag::{MaildirFlag, MaildirFlags},
    maildir::{Maildir, MaildirSubdir},
};
use mail_parser::{Address as MailParserAddress, HeaderValue};

use crate::{
    email::{
        address::Address,
        envelope::{Envelope, normalize_message_id, parse_message_ids},
        flag::{Flag, FlagOp, IanaFlag},
        mailbox::Mailbox,
        search::{eval, query::SearchEmailsQuery},
    },
    maildir::client::MaildirClient,
};

impl MaildirClient {
    /// Lists every Maildir under the configured root, sorted by name.
    /// `with_counts` is ignored (Maildir does not surface counts
    /// cheaply).
    pub fn list_mailboxes(&self, _with_counts: bool) -> Result<Vec<Mailbox>> {
        let mut mailboxes: Vec<Mailbox> = self
            .list_maildirs()?
            .into_iter()
            .map(mailbox_from)
            .collect();
        mailboxes.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(mailboxes)
    }

    /// Lists envelopes from `mailbox`, sorted by `Date:` descending
    /// then paginated. `with_attachment` is always honoured (the body
    /// is parsed regardless).
    pub fn list_envelopes(
        &self,
        mailbox: &str,
        page: Option<u32>,
        page_size: Option<u32>,
        _with_attachment: bool,
    ) -> Result<Vec<Envelope>> {
        let maildir = self.resolve_maildir(Path::new(mailbox))?;
        let entries: Vec<_> = self.list_entries(maildir.clone())?.into_iter().collect();
        let fulls = self.read_entries(&maildir, &entries)?;

        let mut envelopes: Vec<Envelope> = fulls.iter().map(envelope_from_entry).collect();
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
        let maildir = self.resolve_maildir(Path::new(mailbox))?;
        let entries: Vec<_> = self.list_entries(maildir.clone())?.into_iter().collect();
        let fulls = self.read_entries(&maildir, &entries)?;

        let filter = query.and_then(|q| q.filter.as_ref());
        let mut hits: Vec<Envelope> = Vec::new();
        for full in &fulls {
            let envelope = envelope_from_entry(full);
            let keep = match filter {
                Some(filter) => eval::matches_filter(&envelope, full.contents(), filter),
                None => true,
            };
            if keep {
                hits.push(envelope);
            }
        }

        eval::sort_envelopes(&mut hits, query.and_then(|q| q.sort.as_deref()));
        Ok(paginate(hits, page, page_size))
    }

    /// Adds, sets, or removes `flags` on a Maildir id set.
    pub fn store_flags(
        &self,
        mailbox: &str,
        ids: &[&str],
        flags: &[Flag],
        op: FlagOp,
    ) -> Result<()> {
        let maildir = self.resolve_maildir(Path::new(mailbox))?;
        let maildir_flags = flags_to_maildir(flags);

        for id in ids {
            match op {
                FlagOp::Add => self.add_flags(maildir.clone(), *id, maildir_flags.clone())?,
                FlagOp::Set => self.set_flags(maildir.clone(), *id, maildir_flags.clone())?,
                FlagOp::Remove => self.remove_flags(maildir.clone(), *id, maildir_flags.clone())?,
            }
        }

        Ok(())
    }

    /// Reads one message's raw RFC 5322 bytes from `mailbox`.
    pub fn get_message(&self, mailbox: &str, id: &str, seen: bool) -> Result<Vec<u8>> {
        let maildir = self.resolve_maildir(Path::new(mailbox))?;
        let entry = self.get(maildir, id)?;
        let raw = entry.contents().to_vec();

        // NOTE: reading the file never renames it, so `--seen` costs the
        // rename that adds the `S` flag.
        if seen {
            let seen = Flag::from_iana(IanaFlag::Seen);
            self.store_flags(mailbox, &[id], &[seen], FlagOp::Add)?;
        }

        Ok(raw)
    }

    /// Stores `raw` under `mailbox`'s `cur/` with `flags`, returning the
    /// assigned Maildir id.
    pub fn add_message(&self, mailbox: &str, flags: &[Flag], raw: Vec<u8>) -> Result<String> {
        let maildir = self.resolve_maildir(Path::new(mailbox))?;
        let maildir_flags = flags_to_maildir(flags);
        let (id, _path) = self.store(maildir, MaildirSubdir::Cur, maildir_flags, raw)?;
        Ok(id)
    }

    /// Copies every id from `from` to `to`.
    pub fn copy_messages(&self, from: &str, to: &str, ids: &[&str]) -> Result<usize> {
        let source = self.resolve_maildir(Path::new(from))?;
        let target = self.resolve_maildir(Path::new(to))?;

        for id in ids {
            self.copy(*id, source.clone(), target.clone(), None)?;
        }

        Ok(ids.len())
    }

    /// Moves every id from `from` to `to`.
    pub fn move_messages(&self, from: &str, to: &str, ids: &[&str]) -> Result<usize> {
        let source = self.resolve_maildir(Path::new(from))?;
        let target = self.resolve_maildir(Path::new(to))?;

        for id in ids {
            self.r#move(*id, source.clone(), target.clone(), None)?;
        }

        Ok(ids.len())
    }

    /// Permanently deletes `ids` from `mailbox` by unlinking their files.
    pub fn delete_messages(&self, mailbox: &str, ids: &[&str]) -> Result<()> {
        let maildir = self.resolve_maildir(Path::new(mailbox))?;

        for id in ids {
            self.delete_entry(maildir.clone(), *id)?;
        }

        Ok(())
    }
}

/// Converts one [`Maildir`] into the shared [`Mailbox`] shape: `id` is
/// the on-disk path, `name` is the last path segment.
fn mailbox_from(maildir: Maildir) -> Mailbox {
    Mailbox {
        id: maildir.path().to_string(),
        name: maildir.name().unwrap_or("").to_string(),
        total: None,
        unread: None,
    }
}

/// Folds a fully-read Maildir entry into a shared [`Envelope`], parsing
/// the RFC 5322 headers and mapping the flags io-maildir resolved.
fn envelope_from_entry(entry: &MaildirFullEntry) -> Envelope {
    let id = entry.id().unwrap_or_default().to_string();
    let flags = entry.flags().iter().map(flag_from_maildir).collect();
    let size = entry.contents().len() as u64;
    let parsed = entry.parsed();

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

/// Reads a mail-parser msg-id header into the bare ids it names.
///
/// mail-parser usually yields the list `In-Reply-To` holds, but a single
/// id comes back as text, so both shapes are read.
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

/// Maps a shared [`Flag`] to a [`MaildirFlag`]; non-IANA keywords go
/// through [`MaildirFlag::Keyword`] for the dovecot-keywords sidecar.
fn flag_to_maildir(flag: &Flag) -> MaildirFlag {
    match flag.iana() {
        Some(IanaFlag::Seen) => MaildirFlag::Seen,
        Some(IanaFlag::Answered) => MaildirFlag::Replied,
        Some(IanaFlag::Flagged) => MaildirFlag::Flagged,
        Some(IanaFlag::Draft) => MaildirFlag::Draft,
        Some(IanaFlag::Deleted) => MaildirFlag::Trashed,
        Some(IanaFlag::Forwarded) => MaildirFlag::Passed,
        Some(_) | None => MaildirFlag::Keyword(flag.raw().to_string()),
    }
}

/// Shared flag slice to [`MaildirFlags`].
fn flags_to_maildir(flags: &[Flag]) -> MaildirFlags {
    flags.iter().map(flag_to_maildir).collect()
}

/// Maps a [`MaildirFlag`] to a shared [`Flag`]; the inverse of
/// [`flag_to_maildir`].
fn flag_from_maildir(flag: &MaildirFlag) -> Flag {
    match flag {
        MaildirFlag::Seen => Flag::from_iana(IanaFlag::Seen),
        MaildirFlag::Replied => Flag::from_iana(IanaFlag::Answered),
        MaildirFlag::Flagged => Flag::from_iana(IanaFlag::Flagged),
        MaildirFlag::Draft => Flag::from_iana(IanaFlag::Draft),
        MaildirFlag::Trashed => Flag::from_iana(IanaFlag::Deleted),
        MaildirFlag::Passed => Flag::from_iana(IanaFlag::Forwarded),
        MaildirFlag::Keyword(keyword) => Flag::from_raw(keyword),
    }
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
    use io_maildir::path::MaildirFsPath;

    use super::*;

    fn entry(name: &str, contents: &[u8]) -> MaildirFullEntry {
        MaildirFullEntry::from((MaildirFsPath::new(name), contents.to_vec()))
    }

    #[test]
    fn maildir_flags_round_trip_through_the_shared_flag() {
        let flags = [
            MaildirFlag::Seen,
            MaildirFlag::Replied,
            MaildirFlag::Flagged,
            MaildirFlag::Draft,
            MaildirFlag::Trashed,
            MaildirFlag::Passed,
            MaildirFlag::keyword("NonJunk"),
        ];

        for flag in flags {
            assert_eq!(flag_to_maildir(&flag_from_maildir(&flag)), flag);
        }
    }

    #[test]
    fn a_keyword_spelled_like_an_iana_flag_keeps_its_wire_spelling() {
        let flag = flag_from_maildir(&MaildirFlag::keyword("$Junk"));

        assert_eq!(flag.raw(), "$Junk");
        assert_eq!(flag.iana(), Some(IanaFlag::Junk));
    }

    #[test]
    fn a_custom_keyword_carries_no_iana_flag() {
        let flag = flag_from_maildir(&MaildirFlag::keyword("NonJunk"));

        assert_eq!(flag.raw(), "NonJunk");
        assert_eq!(flag.iana(), None);
    }

    #[test]
    fn the_envelope_carries_the_flags_the_entry_was_read_with() {
        let entry = entry(
            "/m/cur/1614632942.M1P2.host:2,RS",
            b"Subject: Hi\r\nFrom: a@x.org\r\n\r\nbody",
        );
        let envelope = envelope_from_entry(&entry);
        let raws: Vec<&str> = envelope.flags.iter().map(Flag::raw).collect();

        assert_eq!(envelope.subject, "Hi");
        assert_eq!(raws, ["\\Seen", "\\Answered"]);
    }
}
