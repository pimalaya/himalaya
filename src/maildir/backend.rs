//! Maildir adapter for the shared cross-protocol client.
//!
//! Thin glue over [`MaildirClient`], which wraps io_maildir's
//! high-level client (`list_maildirs`, `list_entries`, `read_entries`,
//! `add_flags`/`remove_flags`/`set_flags`, `get`, `store`, `copy`,
//! `move`). Each method takes and returns the CLI's shared
//! [`crate::email`] types; the conversion is lifted from the retired
//! io-email Maildir drivers.

use std::{collections::BTreeSet, path::Path};

use anyhow::Result;
use chrono::DateTime;
use io_maildir::{
    entry::types::MaildirFullEntry,
    flag::types::{MaildirFlag, MaildirFlags},
    maildir::types::{Maildir, MaildirSubdir},
    path::FsPath,
};
use mail_parser::Address as MailParserAddress;

use crate::{
    email::{
        address::Address,
        envelope::{Envelope, normalize_message_id},
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
        let entries: Vec<_> = self.list_entries(maildir)?.into_iter().collect();
        let fulls = self.read_entries(&entries)?;

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
        let entries: Vec<_> = self.list_entries(maildir)?.into_iter().collect();
        let fulls = self.read_entries(&entries)?;

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
    pub fn get_message(&self, mailbox: &str, id: &str) -> Result<Vec<u8>> {
        let maildir = self.resolve_maildir(Path::new(mailbox))?;
        let entry = self.get(maildir, id)?;
        Ok(entry.contents().to_vec())
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
    pub fn copy_messages(&self, from: &str, to: &str, ids: &[&str]) -> Result<()> {
        let source = self.resolve_maildir(Path::new(from))?;
        let target = self.resolve_maildir(Path::new(to))?;

        for id in ids {
            self.copy(*id, source.clone(), target.clone(), None)?;
        }

        Ok(())
    }

    /// Moves every id from `from` to `to`.
    pub fn move_messages(&self, from: &str, to: &str, ids: &[&str]) -> Result<()> {
        let source = self.resolve_maildir(Path::new(from))?;
        let target = self.resolve_maildir(Path::new(to))?;

        for id in ids {
            self.r#move(*id, source.clone(), target.clone(), None)?;
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
/// the RFC 5322 headers and reading flags from the filename.
fn envelope_from_entry(entry: &MaildirFullEntry) -> Envelope {
    let id = entry.id().unwrap_or_default().to_string();
    let flags = parse_filename_flags(entry.path());
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

/// IANA flags from a Maildir filename's info section.
fn parse_filename_flags(path: &FsPath) -> BTreeSet<Flag> {
    let Some(name) = path.file_name() else {
        return BTreeSet::new();
    };
    let Some((_, letters)) = name.rsplit_once(',') else {
        return BTreeSet::new();
    };
    letters.chars().filter_map(flag_from_char).collect()
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

/// Maildir info-section letter (S/R/F/D/T/P) to a shared [`Flag`];
/// `None` for letters outside the standard six.
fn flag_from_char(c: char) -> Option<Flag> {
    match c {
        'S' => Some(Flag::from_iana(IanaFlag::Seen)),
        'R' => Some(Flag::from_iana(IanaFlag::Answered)),
        'F' => Some(Flag::from_iana(IanaFlag::Flagged)),
        'D' => Some(Flag::from_iana(IanaFlag::Draft)),
        'T' => Some(Flag::from_iana(IanaFlag::Deleted)),
        'P' => Some(Flag::from_iana(IanaFlag::Forwarded)),
        _ => None,
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
