//! # Maildir backend
//!
//! The Maildir adapter of the unified client, glue over the
//! io-maildir client [`MaildirClient`] wraps.
//!
//! Each method takes and returns the CLI's own [`crate::email`] types, so
//! the work here is converting between those and io-maildir's.

use std::{
    collections::{BTreeSet, HashSet},
    fs,
    path::Path,
};

use crate::{
    email::{
        envelope::{self, Envelope, EnvelopePage, page_follows},
        flag::{Flag, FlagOp, IanaFlag},
        mailbox::{Mailbox, MailboxRole, child_path, renamed_path},
        outcome::Outcome,
        search::{eval, query::SearchEmailsQuery, sort::eval::sort_envelopes},
    },
    error::{ErrorCode, ItemError, headerless_message, message_not_found, missing_message},
    maildir::client::{MaildirClient, refuse_existing, validate_maildir_name},
};
use anyhow::{Result, bail};
use io_maildir::{
    client::{MaildirClientError, MaildirUnreadableEntry},
    entry::{
        MaildirEntry, MaildirFullEntry, get::MaildirEntryGetError, locate::MaildirEntryLocateError,
    },
    flag::{MaildirFlag, MaildirFlags},
    maildir::{Maildir, MaildirSubdir},
};

/// The most message files a Maildir listing holds read at once.
const READ_CHUNK: usize = 64;

impl MaildirClient {
    /// Lists every Maildir under the configured root, sorted by name,
    /// the store's inbox marked with its role. `with_counts` is ignored
    /// (Maildir does not surface counts cheaply). A folder the walk could
    /// not read is an item error.
    pub fn list_mailboxes(&self, _with_counts: bool) -> Result<Outcome<Vec<Mailbox>>> {
        self.ensure_root()?;

        let inbox = self.inbox().map(|inbox| inbox.path().to_string());
        let listing = self.list_maildirs()?;
        let mut mailboxes: Vec<Mailbox> = listing
            .maildirs
            .into_iter()
            .map(|maildir| mailbox_from(self, maildir))
            .map(|mut mailbox| {
                if inbox.as_ref() == Some(&mailbox.id) {
                    mailbox.role = Some(MailboxRole::Inbox);
                }
                mailbox
            })
            .collect();
        mailboxes.sort_by(|a, b| a.name.cmp(&b.name));

        let errors = listing
            .unreadable
            .into_iter()
            .map(|folder| ItemError {
                id: Some(folder.path.to_string()),
                code: Some(ErrorCode::MailboxUnreadable),
                error: String::from("Cannot read the Maildir folder"),
                sources: vec![folder.error.to_string()],
            })
            .collect();

        Ok(Outcome {
            value: mailboxes,
            errors,
        })
    }

    /// Lists envelopes from `mailbox`, sorted by `Date:` descending
    /// then paginated. `with_attachment` is always honoured (the body
    /// is parsed regardless). A file that cannot be read, or holding no
    /// header field, is an item error of the first page.
    pub fn list_envelopes(
        &self,
        mailbox: &str,
        page: Option<u32>,
        page_size: Option<u32>,
        with_attachment: bool,
    ) -> Result<Outcome<Vec<Envelope>>> {
        self.list_envelopes_counted(mailbox, page, page_size, with_attachment)
            .map(|(listed, _)| listed.map(|page| page.envelopes))
    }

    /// Lists envelopes as [`Self::list_envelopes`] does, beside how many
    /// messages the mailbox holds: the entries the listing read.
    pub fn list_envelopes_counted(
        &self,
        mailbox: &str,
        page: Option<u32>,
        page_size: Option<u32>,
        _with_attachment: bool,
    ) -> Result<(Outcome<EnvelopePage>, Option<u64>)> {
        let maildir = self.resolve_maildir(Path::new(mailbox))?;
        let entries: Vec<_> = self.list_entries(maildir.clone())?.into_iter().collect();
        let total = u64::try_from(entries.len()).ok();
        let mut envelopes = Vec::new();
        let mut headerless = Vec::new();

        let unreadable =
            self.visit_entries(&maildir, &entries, |entry| {
                match envelope_from_entry(entry) {
                    Some(envelope) => envelopes.push(envelope),
                    None => headerless.push(entry.id().unwrap_or_default().to_owned()),
                }
            });

        envelopes.sort_by_key(|envelope| std::cmp::Reverse(envelope.date));

        let more = page_follows(envelopes.len() as u64, page, page_size);
        let envelopes = paginate(envelopes, page, page_size);

        let listed = Outcome {
            value: EnvelopePage { envelopes, more },
            errors: entry_errors(unreadable, headerless, page),
        };

        Ok((listed, total))
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
    ) -> Result<Outcome<EnvelopePage>> {
        let maildir = self.resolve_maildir(Path::new(mailbox))?;
        let entries: Vec<_> = self.list_entries(maildir.clone())?.into_iter().collect();
        let filter = query.and_then(|q| q.filter.as_ref());
        let mut hits: Vec<Envelope> = Vec::new();
        let mut headerless = Vec::new();

        let unreadable = self.visit_entries(&maildir, &entries, |full| {
            let Some(envelope) = envelope_from_entry(full) else {
                headerless.push(full.id().unwrap_or_default().to_owned());
                return;
            };
            let keep = match filter {
                Some(filter) => eval::matches_filter(&envelope, None, full.contents(), filter),
                None => true,
            };
            if keep {
                hits.push(envelope);
            }
        });

        sort_envelopes(&mut hits, query.and_then(|q| q.sort.as_deref()));

        let more = page_follows(hits.len() as u64, page, page_size);
        let envelopes = paginate(hits, page, page_size);

        Ok(Outcome {
            value: EnvelopePage { envelopes, more },
            errors: entry_errors(unreadable, headerless, page),
        })
    }

    /// Reads `entries` of `maildir` [`READ_CHUNK`] files at a time, handing
    /// each file read to `visit` and dropping its bytes with its chunk, so a
    /// listing holds a chunk of messages, never the mailbox; returns the
    /// entries it could not read.
    fn visit_entries(
        &self,
        maildir: &Maildir,
        entries: &[MaildirEntry],
        mut visit: impl FnMut(&MaildirFullEntry),
    ) -> Vec<MaildirUnreadableEntry> {
        let mut unreadable = Vec::new();

        for chunk in entries.chunks(READ_CHUNK) {
            let read = self.read_readable_entries(maildir, chunk);
            read.entries.iter().for_each(&mut visit);
            unreadable.extend(read.unreadable);
        }

        unreadable
    }

    /// Adds, sets, or removes `flags` on the messages of `ids` the Maildir
    /// holds, returning how many it found beside the unknown ids.
    pub fn store_flags(
        &self,
        mailbox: &str,
        ids: &[&str],
        flags: &[Flag],
        op: FlagOp,
    ) -> Result<Outcome<usize>> {
        let maildir = self.resolve_maildir(Path::new(mailbox))?;
        let maildir_flags = flags_to_maildir(flags);
        let (found, errors) = self.found(&maildir, ids)?;

        for id in &found {
            match op {
                FlagOp::Add => self.add_flags(maildir.clone(), *id, maildir_flags.clone())?,
                FlagOp::Set => self.set_flags(maildir.clone(), *id, maildir_flags.clone())?,
                FlagOp::Remove => self.remove_flags(maildir.clone(), *id, maildir_flags.clone())?,
            }
        }

        Ok(Outcome {
            value: found.len(),
            errors,
        })
    }

    /// Reads one message's raw RFC 5322 bytes and its flags from
    /// `mailbox`.
    pub fn get_message(
        &self,
        mailbox: &str,
        id: &str,
        seen: bool,
    ) -> Result<(Vec<u8>, BTreeSet<Flag>)> {
        let maildir = self.resolve_maildir(Path::new(mailbox))?;
        let entry = match self.get(maildir.clone(), id) {
            Ok(entry) => entry,
            Err(MaildirClientError::EntryGet(MaildirEntryGetError::Locate(
                MaildirEntryLocateError::NotFound(_),
            ))) => {
                let maildir = self.mailbox_name(&maildir);
                return Err(message_not_found(format!(
                    "Message `{id}` not found in Maildir `{maildir}`"
                )));
            }
            Err(err) => return Err(err.into()),
        };
        let raw = entry.contents().to_vec();
        let mut flags: BTreeSet<Flag> = entry.flags().iter().map(flag_from_maildir).collect();

        // NOTE: reading the file never renames it, so `--seen` costs the
        // rename that adds the `S` flag.
        if seen {
            let seen = Flag::from_iana(IanaFlag::Seen);
            flags.insert(seen.clone());
            self.store_flags(mailbox, &[id], &[seen], FlagOp::Add)?;
        }

        Ok((raw, flags))
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
    /// many it found beside the unknown ids.
    pub fn copy_messages(&self, from: &str, to: &str, ids: &[&str]) -> Result<Outcome<usize>> {
        let source = self.resolve_maildir(Path::new(from))?;
        let target = self.resolve_maildir(Path::new(to))?;
        let (found, errors) = self.found(&source, ids)?;

        for id in &found {
            self.copy(*id, source.clone(), target.clone(), None)?;
        }

        Ok(Outcome {
            value: found.len(),
            errors,
        })
    }

    /// Moves the messages of `ids` `from` holds to `to`, returning how
    /// many it found beside the unknown ids.
    ///
    /// A move within one Maildir changes nothing, the ids kept, rather
    /// than re-delivering each message under a new id.
    pub fn move_messages(&self, from: &str, to: &str, ids: &[&str]) -> Result<Outcome<usize>> {
        let source = self.resolve_maildir(Path::new(from))?;
        let target = self.resolve_maildir(Path::new(to))?;
        let (found, errors) = self.found(&source, ids)?;

        if source.path() != target.path() {
            for id in &found {
                self.r#move(*id, source.clone(), target.clone(), None)?;
            }
        }

        Ok(Outcome {
            value: found.len(),
            errors,
        })
    }

    /// Creates the Maildir `name` at the top of the store, or under
    /// `parent`, returning its id, its path on disk, and its name.
    ///
    /// An existing folder is refused under `mailbox-duplicate`, as IMAP's
    /// `CREATE` refuses one.
    pub fn create_mailbox(&self, name: &str, parent: Option<&str>) -> Result<(String, String)> {
        self.validate_level(name)?;

        let path = match parent {
            Some(parent) => {
                let parent = self.relative_name(&self.resolve_maildir(Path::new(parent))?);
                match parent.is_empty() {
                    true => name.to_string(),
                    false => child_path(&parent, '/', name),
                }
            }
            None => name.to_string(),
        };

        validate_maildir_name(Path::new(&path))?;
        let target = self.store.resolve(&path.as_str().into());
        refuse_existing(Path::new(target.as_str()), &path)?;
        self.create_maildir(path.clone())?;

        let id = self.resolve_maildir(Path::new(&path))?.path().to_string();
        Ok((id, path))
    }

    /// Renames the last level of the Maildir `mailbox` to `name`, its
    /// parent kept, returning its new id and name. The target must not
    /// exist, and the root cannot be renamed.
    pub fn rename_mailbox(&self, mailbox: &str, name: &str) -> Result<(String, String)> {
        self.validate_level(name)?;

        let maildir = self.resolve_maildir(Path::new(mailbox))?;
        if self.is_root(&maildir) {
            let name = self.mailbox_name(&maildir);
            bail!("Cannot rename Maildir `{name}`: it is the store root");
        }

        let from = self.relative_name(&maildir);
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
    /// The root is refused, being the whole store. A Maildir holding
    /// others is refused, naming them, rather than removed with them, as
    /// IMAP servers keeping inferiors refuse it; so is one holding any
    /// directory besides `cur`, `new` and `tmp`, listed or not, which the
    /// removal would take along.
    pub fn delete_mailbox(&self, mailbox: &str) -> Result<String> {
        let maildir = self.resolve_maildir(Path::new(mailbox))?;
        if self.is_root(&maildir) {
            let name = self.mailbox_name(&maildir);
            bail!("Cannot delete Maildir `{name}`: it is the store root");
        }

        let name = self.relative_name(&maildir);
        let prefix = format!("{name}/");

        let mut children: Vec<String> = self
            .list_maildirs()?
            .maildirs
            .iter()
            .map(|maildir| self.relative_name(maildir))
            .filter(|child| child.starts_with(&prefix))
            .collect();

        for entry in fs::read_dir(maildir.path().as_str())? {
            let entry = entry?;
            let level = entry.file_name().to_string_lossy().into_owned();
            let child = child_path(&name, '/', &level);
            let is_dir = fs::metadata(entry.path()).is_ok_and(|m| m.is_dir());
            if is_dir
                && !matches!(level.as_str(), "cur" | "new" | "tmp")
                && !children.contains(&child)
            {
                children.push(child);
            }
        }

        if !children.is_empty() {
            children.sort();
            let children = children.join("`, `");
            bail!("Cannot delete Maildir `{name}`: it holds `{children}`; delete them first");
        }

        self.delete_maildir(name)?;
        Ok(maildir.path().to_string())
    }

    /// The path of `maildir` relative to the root, empty for the root,
    /// which io-maildir resolves names by.
    pub fn relative_name(&self, maildir: &Maildir) -> String {
        self.store
            .relative(maildir.path())
            .map(|name| name.to_string())
            .unwrap_or_default()
    }

    /// Permanently deletes the messages of `ids` `mailbox` holds by
    /// unlinking their files, returning how many it found beside the
    /// unknown ids.
    pub fn delete_messages(&self, mailbox: &str, ids: &[&str]) -> Result<Outcome<usize>> {
        let maildir = self.resolve_maildir(Path::new(mailbox))?;
        let (found, errors) = self.found(&maildir, ids)?;

        for id in &found {
            self.delete_entry(maildir.clone(), *id)?;
        }

        Ok(Outcome {
            value: found.len(),
            errors,
        })
    }

    /// The ids of `ids` `maildir` holds, each entry once, every id checked
    /// before the first write, beside the error of each unknown one.
    fn found<'a>(
        &self,
        maildir: &Maildir,
        ids: &[&'a str],
    ) -> Result<(Vec<&'a str>, Vec<ItemError>)> {
        let (found, missing) = self.locate_all(maildir, ids)?;
        let errors = missing.into_iter().map(missing_message).collect();
        Ok((found, errors))
    }

    /// The ids of `ids` `maildir` holds, each entry once, every id checked
    /// before the first write, as the raw commands want them: one unknown
    /// id refuses the whole set, naming the unknown ones.
    pub fn found_all<'a>(&self, maildir: &Maildir, ids: &[&'a str]) -> Result<Vec<&'a str>> {
        let (found, missing) = self.locate_all(maildir, ids)?;

        if !missing.is_empty() {
            let missing = missing.join("`, `");
            let maildir = self.mailbox_name(maildir);
            return Err(message_not_found(format!(
                "No message changed: `{missing}` not found in Maildir `{maildir}`"
            )));
        }

        Ok(found)
    }

    /// Splits `ids` into the ones `maildir` holds, each entry once, and
    /// the unknown ones.
    fn locate_all<'a>(
        &self,
        maildir: &Maildir,
        ids: &[&'a str],
    ) -> Result<(Vec<&'a str>, Vec<&'a str>)> {
        let located = self.locate_many(maildir.clone(), ids)?;
        let mut paths = HashSet::new();
        let mut unknown = HashSet::new();
        let mut found = Vec::with_capacity(ids.len());
        let mut missing = Vec::new();

        for id in ids {
            match located.get(*id) {
                Some(entry) if paths.insert(entry.path.clone()) => found.push(*id),
                Some(_) => (),
                None if unknown.insert(*id) => missing.push(*id),
                None => (),
            }
        }

        Ok((found, missing))
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
/// the RFC 5322 headers and mapping the flags io-maildir resolved; `None`
/// for a file holding no header field.
fn envelope_from_entry(entry: &MaildirFullEntry) -> Option<Envelope> {
    Some(Envelope {
        id: entry.id().unwrap_or_default().to_string(),
        flags: entry.flags().iter().map(flag_from_maildir).collect(),
        ..envelope::from_raw(entry.contents(), true)?
    })
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

/// The item error of each entry a listing could not read, then of each
/// holding no header field, on its first page only, so a later page does
/// not repeat them.
fn entry_errors(
    unreadable: Vec<MaildirUnreadableEntry>,
    headerless: Vec<String>,
    page: Option<u32>,
) -> Vec<ItemError> {
    if page.unwrap_or(1) > 1 {
        return Vec::new();
    }

    let unreadable = unreadable.into_iter().map(|unreadable| ItemError {
        id: unreadable.entry.id().map(String::from),
        code: Some(ErrorCode::MessageUnreadable),
        error: String::from("Cannot read the message file"),
        sources: vec![unreadable.error.to_string()],
    });

    unreadable
        .chain(headerless.iter().map(|id| headerless_message(id)))
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

#[cfg(test)]
mod tests {
    use std::fs;

    use io_maildir::{entry::MaildirEntry, path::MaildirFsPath};
    use tempfile::tempdir;

    use super::*;
    use crate::{config::maildir::MaildirConfig, error::ErrorCode};

    fn entry(name: &str, contents: &[u8]) -> MaildirFullEntry {
        let entry = MaildirEntry::from_path(MaildirFsPath::new(name));
        MaildirFullEntry::from((entry, contents.to_vec()))
    }

    /// A client over a fresh store holding the Maildir `A` with one
    /// message, returning its id.
    fn fixture() -> (tempfile::TempDir, MaildirClient, String) {
        let dir = tempdir().unwrap();
        let client = MaildirClient::new(MaildirConfig {
            root: dir.path().to_path_buf(),
            keywords: Default::default(),
            layout: Default::default(),
        });
        client.create_mailbox("A", None).unwrap();
        let id = client
            .add_message("A", &[], b"Subject: Hi\r\n\r\nbody".to_vec())
            .unwrap();
        (dir, client, id)
    }

    #[test]
    fn unknown_ids_are_item_errors_before_any_write_and_repeats_count_once() {
        let (_dir, client, id) = fixture();
        client.create_mailbox("B", None).unwrap();
        let flagged = [Flag::from_iana(IanaFlag::Flagged)];
        let missing = |outcome: &Outcome<usize>| -> Vec<Option<String>> {
            outcome
                .errors
                .iter()
                .map(|error| error.id.clone())
                .collect()
        };

        let stored = client
            .store_flags("A", &[&id, "nope", &id], &flagged, FlagOp::Add)
            .unwrap();
        assert_eq!(stored.value, 1);
        assert_eq!(missing(&stored), [Some(String::from("nope"))]);
        assert_eq!(stored.errors[0].code, Some(ErrorCode::MessageNotFound));

        let copied = client.copy_messages("A", "B", &["nope"]).unwrap();
        assert_eq!(copied.value, 0);
        assert_eq!(missing(&copied), [Some(String::from("nope"))]);
        assert!(
            client
                .list_envelopes("B", None, None, false)
                .unwrap()
                .value
                .is_empty()
        );

        let copied = client.copy_messages("A", "B", &[&id, "nope"]).unwrap();
        assert_eq!(copied.value, 1);
        assert_eq!(missing(&copied), [Some(String::from("nope"))]);

        let deleted = client.delete_messages("B", &["nope"]).unwrap();
        assert_eq!(deleted.value, 0);
        assert_eq!(missing(&deleted), [Some(String::from("nope"))]);
    }

    #[test]
    fn a_store_with_dovecot_files_lists_and_a_missing_root_fails() {
        let (dir, client, _) = fixture();
        fs::write(dir.path().join("A/dovecot-uidlist"), b"").unwrap();
        fs::write(dir.path().join("A/dovecot-keywords"), b"").unwrap();

        let names: Vec<_> = client
            .list_mailboxes(false)
            .unwrap()
            .value
            .into_iter()
            .map(|mailbox| mailbox.name)
            .collect();
        assert_eq!(names, ["A"]);

        let missing = MaildirClient::new(MaildirConfig {
            root: dir.path().join("missing"),
            keywords: Default::default(),
            layout: Default::default(),
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

    /// A folder the store holds already, at the top or under a parent, is
    /// refused `mailbox-duplicate` and keeps its messages; a file in the
    /// way is no mailbox, refused uncoded.
    #[test]
    fn an_existing_folder_is_not_created_again() {
        let (dir, client, id) = fixture();
        client.create_mailbox("Kid", Some("A")).unwrap();

        for (name, parent, path) in [("A", None, "A"), ("Kid", Some("A"), "A/Kid")] {
            let err = client.create_mailbox(name, parent).unwrap_err();
            let code = pimalaya_cli::error::code_of::<ErrorCode>(&err);
            assert_eq!(code, Some(ErrorCode::MailboxDuplicate), "{err:#}");
            let sentence = format!("Cannot create mailbox `{path}`: it exists already");
            assert_eq!(err.to_string(), sentence);
        }
        assert!(client.get_message("A", &id, false).is_ok());

        fs::write(dir.path().join("F"), b"").unwrap();
        let err = client.create_mailbox("F", None).unwrap_err();
        assert_eq!(pimalaya_cli::error::code_of::<ErrorCode>(&err), None);
    }

    #[test]
    fn deleting_or_renaming_the_root_leaves_the_store_whole() {
        let (dir, client, id) = fixture();
        for subdir in ["cur", "new", "tmp"] {
            fs::create_dir(dir.path().join(subdir)).unwrap();
        }
        fs::create_dir_all(dir.path().join(".Sent/cur")).unwrap();
        let root = dir.path().to_string_lossy().into_owned();

        for mailbox in [".", "./", "", "INBOX", "A/..", &root] {
            assert!(client.delete_mailbox(mailbox).is_err(), "{mailbox}");
            assert!(client.rename_mailbox(mailbox, "X").is_err(), "{mailbox}");
        }

        assert!(dir.path().join(".Sent/cur").is_dir());
        assert_eq!(
            client
                .store_flags("A", &[&id], &[], FlagOp::Add)
                .unwrap()
                .value,
            1
        );
    }

    #[test]
    fn a_maildir_holding_an_unlisted_folder_is_not_deleted() {
        let (dir, client, _) = fixture();
        fs::create_dir_all(dir.path().join("A/.hidden/cur")).unwrap();
        fs::create_dir_all(dir.path().join("A/Half/cur")).unwrap();

        let err = client.delete_mailbox("A").unwrap_err().to_string();
        assert!(err.contains("A/.hidden") && err.contains("A/Half"), "{err}");
        assert!(dir.path().join("A/.hidden/cur").is_dir());
    }

    #[test]
    fn a_page_counts_the_whole_mailbox() {
        let (_dir, client, _) = fixture();
        client
            .add_message("A", &[], b"Subject: Two\r\n\r\nbody".to_vec())
            .unwrap();

        let (page, total) = client
            .list_envelopes_counted("A", Some(1), Some(1), false)
            .unwrap();

        assert_eq!(page.value.envelopes.len(), 1);
        assert!(page.value.more);
        assert_eq!(total, Some(2));

        let (page, _) = client
            .list_envelopes_counted("A", Some(2), Some(1), false)
            .unwrap();

        assert_eq!(page.value.envelopes.len(), 1);
        assert!(!page.value.more);
    }

    #[test]
    fn a_move_within_one_maildir_keeps_the_id() {
        let (dir, client, id) = fixture();
        let path = dir.path().join("A").to_string_lossy().into_owned();

        assert_eq!(client.move_messages("A", &path, &[&id]).unwrap().value, 1);
        assert_eq!(
            client
                .move_messages("A/", "A", &[&id, "nope"])
                .unwrap()
                .value,
            1
        );

        let envelopes = client.list_envelopes("A", None, None, false).unwrap().value;
        assert_eq!(envelopes.len(), 1);
        assert_eq!(envelopes[0].id, id);
    }

    #[test]
    fn the_raw_lookup_refuses_a_set_holding_an_unknown_id() {
        let (_dir, client, id) = fixture();
        let maildir = client.resolve_maildir(Path::new("A")).unwrap();

        let err = client.found_all(&maildir, &[&id, "nope", &id]).unwrap_err();
        let code = pimalaya_cli::error::code_of::<crate::error::ErrorCode>(&err);
        assert_eq!(code, Some(crate::error::ErrorCode::MessageNotFound));
        assert!(err.to_string().contains("`nope`"), "{err}");

        assert_eq!(client.found_all(&maildir, &[&id, &id]).unwrap(), [&id]);
    }

    #[cfg(unix)]
    #[test]
    fn an_unreadable_message_is_an_item_error_of_the_first_page() {
        use std::os::unix::fs::PermissionsExt;

        let (dir, client, _) = fixture();
        let locked = client
            .add_message("A", &[], b"Subject: Locked\r\n\r\nbody".to_vec())
            .unwrap();
        let path = fs::read_dir(dir.path().join("A/cur"))
            .unwrap()
            .flatten()
            .map(|entry| entry.path())
            .find(|path| path.to_string_lossy().contains(&locked))
            .unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();

        // NOTE: root reads through any mode.
        if fs::read(&path).is_ok() {
            return;
        }

        let listed = client.list_envelopes("A", None, None, false);
        let searched = client.search_envelopes("A", None, None, None, false);
        let second = client.list_envelopes("A", Some(2), Some(1), false);
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();

        let searched = searched.unwrap().map(|page| page.envelopes);
        for outcome in [listed.unwrap(), searched] {
            assert_eq!(outcome.value.len(), 1);
            let [error] = outcome.errors.as_slice() else {
                panic!("{:?}", outcome.errors);
            };
            assert_eq!(error.id.as_deref(), Some(locked.as_str()));
            assert_eq!(error.code, Some(ErrorCode::MessageUnreadable));
        }

        assert!(second.unwrap().errors.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn an_unreadable_folder_is_an_item_error() {
        use std::os::unix::fs::PermissionsExt;

        let (dir, client, _) = fixture();
        let locked = dir.path().join("Locked");
        for subdir in ["cur", "new", "tmp"] {
            fs::create_dir_all(locked.join(subdir)).unwrap();
        }
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();

        // NOTE: root reads through any mode.
        if fs::read_dir(&locked).is_ok() {
            fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
            return;
        }

        let outcome = client.list_mailboxes(false);
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
        let outcome = outcome.unwrap();

        let names: Vec<_> = outcome.value.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, ["A"]);
        let [error] = outcome.errors.as_slice() else {
            panic!("{:?}", outcome.errors);
        };
        let id = error.id.as_deref().unwrap();
        assert!(id.ends_with("Locked"), "{id}");
        assert_eq!(error.code, Some(ErrorCode::MailboxUnreadable));
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
        let envelope = envelope_from_entry(&entry).unwrap();
        let raws: Vec<&str> = envelope.flags.iter().map(Flag::raw).collect();

        assert_eq!(envelope.subject.as_deref(), Some("Hi"));
        assert_eq!(raws, ["\\Seen", "\\Answered"]);
    }

    /// A file holding no header field (empty, bytes, a line of text, bytes
    /// before a colon) is a `message-unreadable` error of the first page,
    /// out of the rows, as a file that cannot be read is.
    #[test]
    fn a_file_holding_no_header_field_is_an_item_error_of_the_first_page() {
        let (dir, client, _) = fixture();
        let files: [(&str, &[u8]); 4] = [
            ("1.empty", b""),
            ("2.bytes", b"\x00\x01\xff\xfe"),
            ("3.text", b"no header here\nsecond line\n"),
            ("4.colon", b"\x01\xfe a: b\n\x7f\n"),
        ];
        for (name, contents) in files {
            fs::write(dir.path().join("A/cur").join(name), contents).unwrap();
        }

        let listed = client.list_envelopes("A", None, None, false).unwrap();
        let searched = client.search_envelopes("A", None, None, None, false);
        let searched = searched.unwrap().map(|page| page.envelopes);
        let second = client.list_envelopes("A", Some(2), Some(1), false);

        for outcome in [listed, searched] {
            let subjects: Vec<&str> = outcome
                .value
                .iter()
                .map(|e| e.subject.as_deref().unwrap_or_default())
                .collect();
            assert_eq!(subjects, ["Hi"]);

            let mut ids: Vec<&str> = outcome
                .errors
                .iter()
                .map(|error| error.id.as_deref().unwrap())
                .collect();
            ids.sort();
            assert_eq!(ids, ["1.empty", "2.bytes", "3.text", "4.colon"]);
            for error in &outcome.errors {
                assert_eq!(error.code, Some(ErrorCode::MessageUnreadable));
            }
        }

        assert!(second.unwrap().errors.is_empty());
    }
}
