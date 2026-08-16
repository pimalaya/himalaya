//! Maildir adapter for the shared cross-protocol client.
//!
//! Thin glue over [`MaildirClient`], which wraps io_maildir's
//! high-level client (`list_maildirs`, `list_entries`, `read_entries`,
//! `add_flags`/`remove_flags`/`set_flags`, `get`, `store`, `copy`,
//! `move`). Each method takes and returns the CLI's shared
//! [`crate::email`] types; the conversion is lifted from the retired
//! io-email Maildir drivers.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use anyhow::Result;
use chrono::DateTime;
use io_maildir::{
    entry::{MaildirFullEntry, headers::extract_keywords_header},
    flag::{KeywordHeader, MaildirFlag, MaildirFlags},
    maildir::{Maildir, MaildirSubdir},
};
use log::warn;
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

    /// Dovecot slot table of `maildir`, empty when the sidecar is
    /// disabled, absent or unreadable.
    fn keyword_table(&self, maildir: &Maildir) -> BTreeMap<char, String> {
        if !self.dovecot_keywords {
            return BTreeMap::new();
        }

        self.load_dovecot_keywords(maildir).unwrap_or_else(|err| {
            warn!("could not load dovecot keywords: {err}");
            BTreeMap::new()
        })
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
        let table = self.keyword_table(&maildir);
        let entries: Vec<_> = self.list_entries(maildir)?.into_iter().collect();
        let fulls = self.read_entries(&entries)?;

        let mut envelopes: Vec<Envelope> = fulls
            .iter()
            .map(|full| envelope_from_entry(full, &table, self.keywords_header))
            .collect();
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
        let table = self.keyword_table(&maildir);
        let entries: Vec<_> = self.list_entries(maildir)?.into_iter().collect();
        let fulls = self.read_entries(&entries)?;

        let filter = query.and_then(|q| q.filter.as_ref());
        let mut hits: Vec<Envelope> = Vec::new();
        for full in &fulls {
            let envelope = envelope_from_entry(full, &table, self.keywords_header);
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

        // Reading the file never renames it; `--seen` adds the `S` flag
        // (a local rename) so the read stays non-mutating by default.
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
/// the RFC 5322 headers and reading flags from the filename.
fn envelope_from_entry(
    entry: &MaildirFullEntry,
    table: &BTreeMap<char, String>,
    header: Option<KeywordHeader>,
) -> Envelope {
    let id = entry.id().unwrap_or_default().to_string();
    let flags = flags_from_entry(entry, table, header);
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

/// Flags of an entry: its info-section letters resolved through the
/// dovecot slot `table`, plus the keywords carried by `header`.
fn flags_from_entry(
    entry: &MaildirFullEntry,
    table: &BTreeMap<char, String>,
    header: Option<KeywordHeader>,
) -> BTreeSet<Flag> {
    let mut flags = MaildirFlags::with_dovecot(entry.path(), table);

    if let Some(header) = header {
        flags.extend_keywords(extract_keywords_header(entry.contents(), header));
    }

    flags.iter().map(flag_from_maildir).collect()
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

    fn raws(flags: &BTreeSet<Flag>) -> Vec<&str> {
        flags.iter().map(Flag::raw).collect()
    }

    #[test]
    fn standard_letters_are_unchanged_without_a_table() {
        let entry = entry("/m/cur/1614632942.M1P2.host:2,FRS", b"");
        let flags = flags_from_entry(&entry, &BTreeMap::new(), None);
        assert_eq!(raws(&flags), ["\\Seen", "\\Answered", "\\Flagged"]);
    }

    #[test]
    fn unknown_letters_are_ignored_without_a_table() {
        let entry = entry("/m/cur/1614632942.M1P2.host:2,Sab", b"");
        let flags = flags_from_entry(&entry, &BTreeMap::new(), None);
        assert_eq!(raws(&flags), ["\\Seen"]);
    }

    #[test]
    fn dovecot_slot_letters_resolve_to_keywords() {
        let table = BTreeMap::from([('a', "NonJunk".to_string())]);
        let entry = entry("/m/cur/1614632942.M1P2.host:2,Sa", b"");
        let flags = flags_from_entry(&entry, &table, None);

        assert_eq!(raws(&flags), ["\\Seen", "NonJunk"]);
        assert!(
            flags
                .iter()
                .any(|flag| flag.raw() == "NonJunk" && flag.iana().is_none())
        );
    }

    #[test]
    fn header_keywords_join_filename_flags() {
        let entry = entry(
            "/m/cur/1614632942.M1P2.host:2,S",
            b"X-Keywords: NonJunk, Work\r\n\r\nbody",
        );
        let flags = flags_from_entry(&entry, &BTreeMap::new(), Some(KeywordHeader::XKeywords));
        assert_eq!(raws(&flags), ["\\Seen", "NonJunk", "Work"]);
    }

    #[test]
    fn header_is_ignored_when_unset() {
        let entry = entry(
            "/m/cur/1614632942.M1P2.host:2,S",
            b"X-Keywords: NonJunk\r\n\r\nbody",
        );
        let flags = flags_from_entry(&entry, &BTreeMap::new(), None);
        assert_eq!(raws(&flags), ["\\Seen"]);
    }

    #[test]
    fn x_label_splits_on_spaces() {
        let entry = entry(
            "/m/cur/1614632942.M1P2.host:2,",
            b"X-Label: work personal\r\n\r\nbody",
        );
        let flags = flags_from_entry(&entry, &BTreeMap::new(), Some(KeywordHeader::XLabel));
        assert_eq!(raws(&flags), ["personal", "work"]);
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
    fn keyword_spelled_like_an_iana_flag_keeps_its_wire_spelling() {
        let table = BTreeMap::from([('a', "$Junk".to_string())]);
        let entry = entry("/m/cur/1614632942.M1P2.host:2,a", b"");
        let flags = flags_from_entry(&entry, &table, None);
        let flag = flags.iter().next().expect("one flag");

        assert_eq!(flag.raw(), "$Junk");
        assert_eq!(flag.iana(), Some(IanaFlag::Junk));
    }

    #[test]
    fn envelope_carries_resolved_keywords() {
        let table = BTreeMap::from([('a', "NonJunk".to_string())]);
        let entry = entry(
            "/m/cur/1614632942.M1P2.host:2,Sa",
            b"Subject: Hi\r\nFrom: a@x.org\r\n\r\nbody",
        );
        let envelope = envelope_from_entry(&entry, &table, None);

        assert_eq!(envelope.subject, "Hi");
        assert!(envelope.flags.iter().any(|flag| flag.raw() == "NonJunk"));
    }
}
