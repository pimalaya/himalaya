//! # m2dir backend
//!
//! The m2dir adapter of the unified client, glue over the
//! io-m2dir client [`M2dirClient`] wraps.
//!
//! m2dir is content-addressed and has no native copy or move, so a copy
//! is a get and a store, its flags written again, and a move deletes
//! after them.
//!
//! io-m2dir joins an id into the `.meta/<id>.flags` path as given, so
//! every id is looked up among the m2dir's entries before a flag write.

use std::{collections::HashMap, path::Path};

use anyhow::{Result, bail};
use io_m2dir::{client::M2dirClientError, entry::M2dirEntry, flag::M2dirFlags, m2dir::M2dir};
use log::warn;

use crate::{
    email::{
        envelope::{self, Envelope},
        flag::{Flag, FlagOp, IanaFlag},
        mailbox::{Mailbox, child_path, validate_level},
        search::{eval, query::SearchEmailsQuery, sort::eval::sort_envelopes},
    },
    m2dir::client::M2dirClient,
};

impl M2dirClient {
    /// Resolves a shared mailbox argument into an opened [`M2dir`].
    ///
    /// An absolute path opens directly, and a relative name is resolved
    /// under the store root first, percent-encoded as the spec wants,
    /// io-m2dir taking a filesystem path alone.
    ///
    /// That is what lets a unified command address a folder by name or by
    /// id, as the raw `m2dir` commands and the Maildir backend do.
    fn resolve_m2dir(&self, mailbox: &str) -> Result<M2dir> {
        if Path::new(mailbox).is_absolute() {
            return Ok(self.open_m2dir(mailbox)?);
        }

        let store = self.open_store()?;
        let path = store.resolve_folder_path(mailbox)?;
        Ok(self.open_m2dir(path)?)
    }

    /// The name a folder is shown and selected by: its path relative to
    /// the store root, decoded, so `-m` takes back what a listing prints.
    pub fn m2dir_name(&self, m2dir: &M2dir) -> String {
        let path = m2dir.path();
        self.open_store()
            .ok()
            .and_then(|store| store.decode_folder_name(path))
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| path.file_name().unwrap_or("").to_string())
    }

    /// Lists every m2dir under the configured root, sorted by name.
    /// `with_counts` is ignored (m2dir does not surface counts cheaply).
    pub fn list_mailboxes(&self, _with_counts: bool) -> Result<Vec<Mailbox>> {
        let mut mailboxes: Vec<Mailbox> = self
            .list_m2dirs()?
            .into_iter()
            .map(|m2dir| mailbox_from(self, m2dir))
            .collect();
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
        _with_attachment: bool,
    ) -> Result<Vec<Envelope>> {
        let m2dir = self.resolve_m2dir(mailbox)?;

        let mut envelopes: Vec<Envelope> = self
            .read_listed(&m2dir)?
            .into_iter()
            .map(|(envelope, _)| envelope)
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
        let m2dir = self.resolve_m2dir(mailbox)?;

        let filter = query.and_then(|q| q.filter.as_ref());
        let mut hits = Vec::new();
        for (envelope, bytes) in self.read_listed(&m2dir)? {
            let keep = match filter {
                Some(filter) => eval::matches_filter(&envelope, None, &bytes, filter),
                None => true,
            };
            if keep {
                hits.push(envelope);
            }
        }

        sort_envelopes(&mut hits, query.and_then(|q| q.sort.as_deref()));
        Ok(paginate(hits, page, page_size))
    }

    /// The envelope and bytes of every entry of `m2dir`.
    ///
    /// An entry whose bytes no longer match the checksum its name carries
    /// is skipped with a warning, so one edited file does not hide the
    /// rest of the folder; reading it by id still fails.
    fn read_listed(&self, m2dir: &M2dir) -> Result<Vec<(Envelope, Vec<u8>)>> {
        let entries = self.list_entries(m2dir.clone())?;
        let mut listed = Vec::with_capacity(entries.len());

        for entry in &entries {
            let bytes = match self.read_entry(entry) {
                Ok(bytes) => bytes,
                Err(M2dirClientError::Parse(err)) => {
                    warn!("skip m2dir entry {}: {err}", entry.path());
                    continue;
                }
                Err(err) => return Err(err.into()),
            };
            let flags = self.read_flags(m2dir, entry.id())?;
            listed.push((envelope_from(entry, &flags, &bytes), bytes));
        }

        Ok(listed)
    }

    /// Adds, sets, or removes `flags` on the messages of `ids` the m2dir
    /// holds, returning how many it found.
    ///
    /// A removal drops the earlier IMAP spelling (`\Seen`) of a flag too,
    /// which Himalaya wrote before it followed the m2dir spec.
    pub fn store_flags(
        &self,
        mailbox: &str,
        ids: &[&str],
        flags: &[Flag],
        op: FlagOp,
    ) -> Result<usize> {
        let m2dir = self.resolve_m2dir(mailbox)?;
        let found = self.found(&m2dir, ids)?;

        let mut m2dir_flags = flags_to_m2dir(flags);
        if let FlagOp::Remove = op {
            for flag in flags {
                m2dir_flags.insert(flag.raw());
            }
        }

        for entry in &found {
            match op {
                FlagOp::Add => self.add_flags(&m2dir, entry.id(), m2dir_flags.clone())?,
                FlagOp::Set => self.set_flags(&m2dir, entry.id(), m2dir_flags.clone())?,
                FlagOp::Remove => self.remove_flags(&m2dir, entry.id(), m2dir_flags.clone())?,
            }
        }

        Ok(found.len())
    }

    /// Reads one message's raw RFC 5322 bytes from `mailbox`.
    pub fn get_message(&self, mailbox: &str, id: &str, seen: bool) -> Result<Vec<u8>> {
        let m2dir = self.resolve_m2dir(mailbox)?;
        let (_entry, bytes) = self.get(m2dir, id)?;

        // NOTE: reading the bytes never touches the sidecar, so `--seen`
        // costs a separate write of the `S` flag.
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

    /// Copies the messages of `ids` `from` holds to `to`, their flags with
    /// them, returning how many it found.
    pub fn copy_messages(&self, from: &str, to: &str, ids: &[&str]) -> Result<usize> {
        let source = self.resolve_m2dir(from)?;
        let target = self.resolve_m2dir(to)?;
        let found = self.found(&source, ids)?;

        for entry in &found {
            self.copy_entry(&source, &target, entry)?;
        }

        Ok(found.len())
    }

    /// Moves the messages of `ids` `from` holds to `to`, their flags with
    /// them: a copy, then a delete of the source. Returns how many it
    /// found.
    pub fn move_messages(&self, from: &str, to: &str, ids: &[&str]) -> Result<usize> {
        let source = self.resolve_m2dir(from)?;
        let target = self.resolve_m2dir(to)?;
        let found = self.found(&source, ids)?;

        for entry in &found {
            self.copy_entry(&source, &target, entry)?;
            self.delete_entry(source.clone(), entry.id())?;
        }

        Ok(found.len())
    }

    /// Stores the bytes of `entry` in `target` and writes its flags there,
    /// returning the copy's id.
    fn copy_entry(&self, source: &M2dir, target: &M2dir, entry: &M2dirEntry) -> Result<String> {
        let bytes = self.read_entry(entry)?;
        let flags = self.read_flags(source, entry.id())?;
        let copy = self.store(target.clone(), bytes)?;

        if !flags.is_empty() {
            self.set_flags(target, copy.id(), flags)?;
        }

        Ok(copy.id().to_string())
    }

    /// Creates the folder `name` at the top of the store, or under
    /// `parent`, the store initialised first when it does not exist yet,
    /// returning its id, its path on disk, and its name.
    ///
    /// An existing folder is refused, as IMAP's `CREATE` refuses one.
    pub fn create_mailbox(&self, name: &str, parent: Option<&str>) -> Result<(String, String)> {
        validate_m2dir_level(name)?;

        let name = match parent {
            Some(parent) => child_path(&self.m2dir_name(&self.resolve_m2dir(parent)?), '/', name),
            None => name.to_string(),
        };

        let store = self.init_store()?;
        let path = store.resolve_folder_path(&name)?;
        if self.open_m2dir(path).is_ok() {
            bail!("Cannot create m2dir `{name}`: it already exists");
        }

        let m2dir = self.create_m2dir(&name)?;
        Ok((m2dir.path().as_str().to_string(), name))
    }

    /// Deletes the folder `mailbox` and every message in it, returning its
    /// id.
    ///
    /// A folder holding others is refused, naming them, rather than
    /// removed with them, as IMAP servers keeping inferiors refuse it.
    pub fn delete_mailbox(&self, mailbox: &str) -> Result<String> {
        let m2dir = self.resolve_m2dir(mailbox)?;
        let prefix = format!("{}/", m2dir.path());

        let children: Vec<String> = self
            .list_m2dirs()?
            .iter()
            .filter(|child| child.path().as_str().starts_with(&prefix))
            .map(|child| self.m2dir_name(child))
            .collect();

        if !children.is_empty() {
            let name = self.m2dir_name(&m2dir);
            let children = children.join("`, `");
            bail!("Cannot delete m2dir `{name}`: it holds `{children}`; delete them first");
        }

        self.delete_m2dir(m2dir.path().clone())?;
        Ok(m2dir.path().as_str().to_string())
    }

    /// Permanently deletes the messages of `ids` `mailbox` holds by
    /// unlinking their files, returning how many it found.
    pub fn delete_messages(&self, mailbox: &str, ids: &[&str]) -> Result<usize> {
        let m2dir = self.resolve_m2dir(mailbox)?;
        let found = self.found(&m2dir, ids)?;

        for entry in &found {
            self.delete_entry(m2dir.clone(), entry.id())?;
        }

        Ok(found.len())
    }

    /// The entries of `ids` `m2dir` holds, each once, every id checked
    /// before the first write. An unknown one is skipped with a warning.
    pub fn found(&self, m2dir: &M2dir, ids: &[&str]) -> Result<Vec<M2dirEntry>> {
        let mut entries: HashMap<String, M2dirEntry> = self
            .list_entries(m2dir.clone())?
            .into_iter()
            .map(|entry| (entry.id().to_string(), entry))
            .collect();

        let mut found = Vec::with_capacity(ids.len());
        for id in ids {
            match entries.remove(*id) {
                Some(entry) => found.push(entry),
                None if found.iter().any(|entry| entry.id() == *id) => {}
                None => warn!("skip message `{id}`: not found in {}", m2dir.path()),
            }
        }

        Ok(found)
    }

    /// The entries of `ids` `m2dir` holds, refusing the set before any
    /// write when one is unknown, for the raw commands.
    pub fn entries(&self, m2dir: &M2dir, ids: &[&str]) -> Result<Vec<M2dirEntry>> {
        let found = self.found(m2dir, ids)?;
        let unknown: Vec<&str> = ids
            .iter()
            .copied()
            .filter(|id| !found.iter().any(|entry| entry.id() == *id))
            .collect();

        if !unknown.is_empty() {
            let unknown = unknown.join("`, `");
            bail!("Message `{unknown}` not found in m2dir {}", m2dir.path());
        }

        Ok(found)
    }
}

/// Rejects a NAME that cannot be one level of an m2store: one holding a
/// separator io-m2dir nests on (`/`, `\`), or starting with a dot, which
/// the listing reads as hidden.
pub fn validate_m2dir_level(name: &str) -> Result<()> {
    validate_level(name, '/')?;
    validate_level(name, '\\')?;

    if name.starts_with('.') {
        bail!("m2dir folder name `{name}` must not start with a dot: it would be hidden");
    }

    Ok(())
}

/// Converts one [`M2dir`] into the shared [`Mailbox`] shape: `id` is the
/// on-disk path, `name` the decoded path relative to the store root.
fn mailbox_from(client: &M2dirClient, m2dir: M2dir) -> Mailbox {
    Mailbox {
        name: client.m2dir_name(&m2dir),
        id: m2dir.path().as_str().to_string(),
        role: None,
        total: None,
        unread: None,
    }
}

/// Folds an entry, its meta flags and its bytes into an [`Envelope`],
/// attachments marked as on the other local stores, the bytes being read.
fn envelope_from(entry: &M2dirEntry, meta: &M2dirFlags, raw: &[u8]) -> Envelope {
    Envelope {
        id: entry.id().to_string(),
        flags: meta.iter().map(flag_from_meta_line).collect(),
        ..envelope::from_raw(raw, true)
    }
}

/// One `.meta/<id>.flags` line to a shared [`Flag`], whitespace trimmed:
/// the m2dir spelling (`$seen`) and the IMAP one Himalaya wrote before
/// (`\Seen`) both read as the system flag.
fn flag_from_meta_line(line: &str) -> Flag {
    let line = line.trim();
    let iana = match line {
        "$seen" | "\\Seen" => IanaFlag::Seen,
        "$answered" | "\\Answered" => IanaFlag::Answered,
        "$flagged" | "\\Flagged" => IanaFlag::Flagged,
        "$draft" | "\\Draft" => IanaFlag::Draft,
        "$Deleted" | "\\Deleted" => IanaFlag::Deleted,
        _ => return Flag::from_raw(line),
    };
    Flag::from_iana(iana)
}

/// One shared [`Flag`] to its m2dir line: the system flags in the m2dir
/// spec's `$` spelling, keywords as they are.
fn flag_to_m2dir(flag: &Flag) -> &str {
    match flag.iana() {
        Some(IanaFlag::Seen) => "$seen",
        Some(IanaFlag::Answered) => "$answered",
        Some(IanaFlag::Flagged) => "$flagged",
        Some(IanaFlag::Draft) => "$draft",
        Some(IanaFlag::Deleted) => "$Deleted",
        _ => flag.raw(),
    }
}

/// Shared flag slice to [`M2dirFlags`], one line per flag.
fn flags_to_m2dir(flags: &[Flag]) -> M2dirFlags {
    flags.iter().map(flag_to_m2dir).collect()
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

    use tempfile::tempdir;

    use super::*;
    use crate::config::M2dirConfig;

    /// A client over a fresh store holding the folder `A` with one
    /// message, returning its id.
    fn fixture() -> (tempfile::TempDir, M2dirClient, String) {
        let dir = tempdir().unwrap();
        let client = M2dirClient::new(M2dirConfig {
            root: dir.path().to_path_buf(),
        });
        client.create_mailbox("A", None).unwrap();
        let id = client
            .add_message("A", &[], b"Subject: Hi\r\n\r\nbody".to_vec())
            .unwrap();
        (dir, client, id)
    }

    #[test]
    fn a_flag_write_never_reaches_a_path_an_id_spells() {
        let (dir, client, id) = fixture();
        let seen = [Flag::from_iana(IanaFlag::Seen)];

        let count = client
            .store_flags("A", &[&id, "../../escape", "nope", &id], &seen, FlagOp::Add)
            .unwrap();

        assert_eq!(count, 1);
        assert!(!dir.path().join("escape.flags").exists());
        assert!(!dir.path().join("A/.meta/nope.flags").exists());
    }

    #[test]
    fn system_flags_are_written_in_the_m2dir_spelling_and_read_in_both() {
        let (dir, client, id) = fixture();
        let flags = [
            Flag::from_iana(IanaFlag::Seen),
            Flag::from_iana(IanaFlag::Flagged),
            Flag::from_iana(IanaFlag::Junk),
        ];
        client
            .store_flags("A", &[&id], &flags, FlagOp::Add)
            .unwrap();

        let path = dir.path().join(format!("A/.meta/{id}.flags"));
        let meta = fs::read_to_string(&path).unwrap();
        let mut lines: Vec<&str> = meta.lines().collect();
        lines.sort();
        assert_eq!(lines, ["$Junk", "$flagged", "$seen"]);

        fs::write(&path, "\\Seen\n\\Answered\n").unwrap();
        let envelopes = client.list_envelopes("A", None, None, false).unwrap();
        let flags: Vec<_> = envelopes[0].flags.iter().map(Flag::iana).collect();
        assert_eq!(flags, [Some(IanaFlag::Seen), Some(IanaFlag::Answered)]);

        let seen = [Flag::from_iana(IanaFlag::Seen)];
        client
            .store_flags("A", &[&id], &seen, FlagOp::Remove)
            .unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap().trim(), "\\Answered");
    }

    #[test]
    fn a_copy_and_a_move_keep_the_flags() {
        let (_dir, client, id) = fixture();
        client.create_mailbox("B", None).unwrap();
        let flagged = [Flag::from_iana(IanaFlag::Flagged)];
        client
            .store_flags("A", &[&id], &flagged, FlagOp::Add)
            .unwrap();

        assert_eq!(client.copy_messages("A", "B", &[&id, "nope"]).unwrap(), 1);
        assert_eq!(client.move_messages("A", "B", &[&id]).unwrap(), 1);

        let envelopes = client.list_envelopes("B", None, None, false).unwrap();
        assert_eq!(envelopes.len(), 2);
        for envelope in envelopes {
            assert!(envelope.flags.iter().any(Flag::is_flagged));
            assert_eq!(envelope.has_attachment, Some(false));
        }
    }

    #[test]
    fn an_entry_whose_bytes_changed_does_not_hide_the_folder() {
        let (dir, client, id) = fixture();
        client
            .add_message("A", &[], b"Subject: Other\r\n\r\nbody".to_vec())
            .unwrap();

        let entry = fs::read_dir(dir.path().join("A"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .find(|path| path.to_string_lossy().ends_with(&id))
            .unwrap();
        fs::write(&entry, "Subject: Edited\r\n\r\nbody").unwrap();

        let envelopes = client.list_envelopes("A", None, None, false).unwrap();
        let subjects: Vec<_> = envelopes.iter().map(|e| e.subject.as_str()).collect();
        assert_eq!(subjects, ["Other"]);
    }

    #[test]
    fn a_folder_is_named_by_its_decoded_path_and_nests_under_a_parent() {
        let (_dir, client, _) = fixture();
        client.create_mailbox("Été", None).unwrap();
        client.create_mailbox("Sub", Some("A")).unwrap();

        let names: Vec<_> = client
            .list_mailboxes(false)
            .unwrap()
            .into_iter()
            .map(|mailbox| mailbox.name)
            .collect();
        assert_eq!(names, ["A", "A/Sub", "Été"]);

        assert!(client.list_envelopes("A/Sub", None, None, false).is_ok());
        assert!(client.create_mailbox("Sub", Some("A")).is_err());
        assert!(client.create_mailbox("x/y", None).is_err());
        assert!(client.delete_mailbox("A").is_err());
    }
}
