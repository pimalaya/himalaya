//! # Maildir backend
//!
//! The Maildir adapter of the unified client, glue over the
//! io-maildir client [`MaildirClient`] wraps.
//!
//! Each method takes and returns the CLI's own [`crate::email`] types, so
//! the work here is converting between those and io-maildir's.

use std::{collections::HashSet, path::Path};

use anyhow::{Result, bail};
use io_maildir::{
    client::MaildirClientError,
    entry::{MaildirFullEntry, locate::MaildirEntryLocateError},
    flag::{MaildirFlag, MaildirFlags},
    maildir::{Maildir, MaildirSubdir},
};
use log::warn;

use crate::{
    email::{
        envelope::{self, Envelope},
        flag::{Flag, FlagOp, IanaFlag},
        mailbox::{Mailbox, child_path, renamed_path},
        search::{eval, query::SearchEmailsQuery, sort::eval::sort_envelopes},
    },
    maildir::client::{MaildirClient, validate_maildir_level, validate_maildir_name},
};

impl MaildirClient {
    /// Lists every Maildir under the configured root, sorted by name.
    /// `with_counts` is ignored (Maildir does not surface counts
    /// cheaply).
    pub fn list_mailboxes(&self, _with_counts: bool) -> Result<Vec<Mailbox>> {
        self.ensure_root()?;

        let mut mailboxes: Vec<Mailbox> = self
            .list_maildirs()?
            .into_iter()
            .map(|maildir| mailbox_from(self, maildir))
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
                Some(filter) => eval::matches_filter(&envelope, None, full.contents(), filter),
                None => true,
            };
            if keep {
                hits.push(envelope);
            }
        }

        sort_envelopes(&mut hits, query.and_then(|q| q.sort.as_deref()));
        Ok(paginate(hits, page, page_size))
    }

    /// Adds, sets, or removes `flags` on the messages of `ids` the Maildir
    /// holds, returning how many it found.
    pub fn store_flags(
        &self,
        mailbox: &str,
        ids: &[&str],
        flags: &[Flag],
        op: FlagOp,
    ) -> Result<usize> {
        let maildir = self.resolve_maildir(Path::new(mailbox))?;
        let maildir_flags = flags_to_maildir(flags);
        let found = self.found(&maildir, ids)?;

        for id in &found {
            match op {
                FlagOp::Add => self.add_flags(maildir.clone(), *id, maildir_flags.clone())?,
                FlagOp::Set => self.set_flags(maildir.clone(), *id, maildir_flags.clone())?,
                FlagOp::Remove => self.remove_flags(maildir.clone(), *id, maildir_flags.clone())?,
            }
        }

        Ok(found.len())
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

    /// Copies the messages of `ids` `from` holds to `to`, returning how
    /// many it found.
    pub fn copy_messages(&self, from: &str, to: &str, ids: &[&str]) -> Result<usize> {
        let source = self.resolve_maildir(Path::new(from))?;
        let target = self.resolve_maildir(Path::new(to))?;
        let found = self.found(&source, ids)?;

        for id in &found {
            self.copy(*id, source.clone(), target.clone(), None)?;
        }

        Ok(found.len())
    }

    /// Moves the messages of `ids` `from` holds to `to`, returning how
    /// many it found.
    pub fn move_messages(&self, from: &str, to: &str, ids: &[&str]) -> Result<usize> {
        let source = self.resolve_maildir(Path::new(from))?;
        let target = self.resolve_maildir(Path::new(to))?;
        let found = self.found(&source, ids)?;

        for id in &found {
            self.r#move(*id, source.clone(), target.clone(), None)?;
        }

        Ok(found.len())
    }

    /// Creates the Maildir `name` at the top of the store, or under
    /// `parent`, returning its id, its path on disk, and its name.
    ///
    /// An existing Maildir is refused, as IMAP's `CREATE` refuses one.
    pub fn create_mailbox(&self, name: &str, parent: Option<&str>) -> Result<(String, String)> {
        validate_maildir_level(name)?;

        let path = match parent {
            Some(parent) => {
                let parent = self.resolve_maildir(Path::new(parent))?;
                child_path(&self.mailbox_name(&parent), '/', name)
            }
            None => name.to_string(),
        };

        validate_maildir_name(Path::new(&path))?;
        if self.load_maildir(path.clone()).is_ok() {
            bail!("Cannot create Maildir `{path}`: it already exists");
        }
        self.create_maildir(path.clone())?;

        let id = self.resolve_maildir(Path::new(&path))?.path().to_string();
        Ok((id, path))
    }

    /// Renames the last level of the Maildir `mailbox` to `name`, its
    /// parent kept, returning its new id and name. The target must not
    /// exist.
    pub fn rename_mailbox(&self, mailbox: &str, name: &str) -> Result<(String, String)> {
        validate_maildir_level(name)?;

        let from = self.mailbox_name(&self.resolve_maildir(Path::new(mailbox))?);
        let to = renamed_path(&from, '/', name);

        validate_maildir_name(Path::new(&to))?;
        let target = self.store.resolve(&to.as_str().into());
        if Path::new(target.as_str()).exists() {
            bail!("Cannot rename Maildir `{from}` to `{to}`: `{to}` already exists");
        }
        self.rename_maildir(from, to.clone())?;

        let id = self.resolve_maildir(Path::new(&to))?.path().to_string();
        Ok((id, to))
    }

    /// Deletes the Maildir `mailbox` and every message in it, returning
    /// its id.
    ///
    /// A Maildir holding others is refused, naming them, rather than
    /// removed with them, as IMAP servers keeping inferiors refuse it.
    pub fn delete_mailbox(&self, mailbox: &str) -> Result<String> {
        let maildir = self.resolve_maildir(Path::new(mailbox))?;
        let name = self.mailbox_name(&maildir);
        let prefix = format!("{name}/");

        let children: Vec<String> = self
            .list_maildirs()?
            .iter()
            .map(|maildir| self.mailbox_name(maildir))
            .filter(|child| child.starts_with(&prefix))
            .collect();

        if !children.is_empty() {
            let children = children.join("`, `");
            bail!("Cannot delete Maildir `{name}`: it holds `{children}`; delete them first");
        }

        self.delete_maildir(name)?;
        Ok(maildir.path().to_string())
    }

    /// Permanently deletes the messages of `ids` `mailbox` holds by
    /// unlinking their files, returning how many it found.
    pub fn delete_messages(&self, mailbox: &str, ids: &[&str]) -> Result<usize> {
        let maildir = self.resolve_maildir(Path::new(mailbox))?;
        let found = self.found(&maildir, ids)?;

        for id in &found {
            self.delete_entry(maildir.clone(), *id)?;
        }

        Ok(found.len())
    }

    /// The ids of `ids` `maildir` holds, each entry once, every id checked
    /// before the first write. An unknown one is skipped with a warning.
    fn found<'a>(&self, maildir: &Maildir, ids: &[&'a str]) -> Result<Vec<&'a str>> {
        let mut paths = HashSet::new();
        let mut found = Vec::with_capacity(ids.len());

        for id in ids {
            match self.locate(maildir.clone(), *id) {
                Ok((path, _, _)) => {
                    if paths.insert(path) {
                        found.push(*id);
                    }
                }
                Err(MaildirClientError::EntryLocate(MaildirEntryLocateError::NotFound(_))) => {
                    warn!("skip message `{id}`: not found in {}", maildir.path());
                }
                Err(err) => return Err(err.into()),
            }
        }

        Ok(found)
    }
}

/// Converts one [`Maildir`] into the shared [`Mailbox`] shape: `id` is
/// the on-disk path, `name` the [`MaildirClient::mailbox_name`].
fn mailbox_from(client: &MaildirClient, maildir: Maildir) -> Mailbox {
    Mailbox {
        name: client.mailbox_name(&maildir),
        id: maildir.path().to_string(),
        role: None,
        total: None,
        unread: None,
    }
}

/// Folds a fully-read Maildir entry into a shared [`Envelope`], reading
/// the RFC 5322 headers and mapping the flags io-maildir resolved.
fn envelope_from_entry(entry: &MaildirFullEntry) -> Envelope {
    Envelope {
        id: entry.id().unwrap_or_default().to_string(),
        flags: entry.flags().iter().map(flag_from_maildir).collect(),
        ..envelope::from_raw(entry.contents(), true)
    }
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
    use std::fs;

    use io_maildir::path::MaildirFsPath;
    use tempfile::tempdir;

    use super::*;
    use crate::config::MaildirConfig;

    fn entry(name: &str, contents: &[u8]) -> MaildirFullEntry {
        MaildirFullEntry::from((MaildirFsPath::new(name), contents.to_vec()))
    }

    /// A client over a fresh store holding the Maildir `A` with one
    /// message, returning its id.
    fn fixture() -> (tempfile::TempDir, MaildirClient, String) {
        let dir = tempdir().unwrap();
        let client = MaildirClient::new(MaildirConfig {
            root: dir.path().to_path_buf(),
            keywords: Default::default(),
        });
        client.create_mailbox("A", None).unwrap();
        let id = client
            .add_message("A", &[], b"Subject: Hi\r\n\r\nbody".to_vec())
            .unwrap();
        (dir, client, id)
    }

    #[test]
    fn unknown_ids_are_skipped_before_any_write_and_repeats_count_once() {
        let (_dir, client, id) = fixture();
        client.create_mailbox("B", None).unwrap();
        let flagged = [Flag::from_iana(IanaFlag::Flagged)];

        let count = client
            .store_flags("A", &[&id, "nope", &id], &flagged, FlagOp::Add)
            .unwrap();
        assert_eq!(count, 1);

        assert_eq!(client.copy_messages("A", "B", &["nope"]).unwrap(), 0);
        assert!(
            client
                .list_envelopes("B", None, None, false)
                .unwrap()
                .is_empty()
        );
        assert_eq!(client.copy_messages("A", "B", &[&id, "nope"]).unwrap(), 1);
        assert_eq!(client.delete_messages("B", &["nope"]).unwrap(), 0);
    }

    #[test]
    fn a_store_with_dovecot_files_lists_and_a_missing_root_fails() {
        let (dir, client, _) = fixture();
        fs::write(dir.path().join("A/dovecot-uidlist"), b"").unwrap();
        fs::write(dir.path().join("A/dovecot-keywords"), b"").unwrap();

        let names: Vec<_> = client
            .list_mailboxes(false)
            .unwrap()
            .into_iter()
            .map(|mailbox| mailbox.name)
            .collect();
        assert_eq!(names, ["A"]);

        let missing = MaildirClient::new(MaildirConfig {
            root: dir.path().join("missing"),
            keywords: Default::default(),
        });
        let err = missing.list_mailboxes(false).unwrap_err().to_string();
        assert!(err.contains("does not exist"), "{err}");
    }

    #[test]
    fn a_mailbox_write_keeps_one_listable_level() {
        let (dir, client, _) = fixture();

        for name in ["cur", ".hidden", "x/y"] {
            assert!(client.create_mailbox(name, None).is_err(), "{name}");
        }
        assert!(client.create_mailbox("A", None).is_err());
        assert!(!dir.path().join("cur").exists());

        let (id, name) = client.create_mailbox("Kid", Some("A")).unwrap();
        assert_eq!(name, "A/Kid");
        assert_eq!(id, dir.path().join("A/Kid").to_string_lossy());

        client.create_mailbox("B", None).unwrap();
        assert!(client.rename_mailbox("B", "A").is_err());
        assert!(client.rename_mailbox("B", "x/y").is_err());
        let (_, renamed) = client.rename_mailbox("A/Kid", "Kid2").unwrap();
        assert_eq!(renamed, "A/Kid2");

        let err = client.delete_mailbox("A").unwrap_err().to_string();
        assert!(err.contains("A/Kid2"), "{err}");
        assert!(dir.path().join("A/Kid2/cur").is_dir());
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
