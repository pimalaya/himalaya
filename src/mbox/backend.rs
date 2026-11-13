//! # mbox backend
//!
//! The mbox adapter of the unified client, glue over the
//! io-mbox client [`MboxClient`] wraps.
//!
//! A mailbox is one mbox file and a message is addressed by its io-mbox
//! content id, which a flag write, by Himalaya or any other client, does
//! not change. Every operation first syncs the file's index through the
//! [`crate::mbox::cache`], and a listing parses only the messages it has
//! not seen before.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::Path,
};

use anyhow::{Context, Result, anyhow, bail};
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
    mbox::{create::MboxCreate, delete::MboxDelete, rename::MboxRename},
    path::{MboxFsPath, MboxPath},
    store::{INBOX, SBD},
};
use log::warn;

use crate::{
    email::{
        envelope::{self, Envelope},
        flag::{Flag, FlagOp, IanaFlag},
        mailbox::{Mailbox, MailboxRole, child_path, renamed_path},
        search::{eval, query::SearchEmailsQuery, sort::eval::sort_envelopes},
    },
    error::message_not_found,
    mbox::{
        cache::MboxCache,
        client::{MboxClient, validate_mbox_name},
    },
};

/// The fields the store owns: the flag fields, which an append writes from
/// the flags it is given, and Thunderbird's, which a flag write keeps in
/// step.
const STORE_FIELDS: [&str; 6] = [
    "Status",
    "X-Status",
    "X-Keywords",
    "X-Mozilla-Status",
    "X-Mozilla-Status2",
    "X-Mozilla-Keys",
];

impl MboxClient {
    /// Lists the mbox files of the store, the spool as `INBOX`.
    ///
    /// `with_counts` syncs the index of every file, which scans a file the
    /// cache does not know yet.
    pub fn list_mailboxes(&self, with_counts: bool) -> Result<Vec<Mailbox>> {
        self.check_root()?;
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
        let path = self.open_mbox(mailbox)?;
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
        let path = self.open_mbox(mailbox)?;
        let filter = query.and_then(|q| q.filter.as_ref());

        let mut hits: Vec<Envelope> = Vec::new();
        for (envelope, contents) in self.envelopes(&path, filter.is_some())? {
            let keep = match (filter, contents) {
                (Some(filter), Some(contents)) => {
                    eval::matches_filter(&envelope, None, &contents, filter)
                }
                _ => true,
            };
            if keep {
                hits.push(envelope);
            }
        }

        sort_envelopes(&mut hits, query.and_then(|q| q.sort.as_deref()));
        Ok(paginate(hits, page, page_size))
    }

    /// Adds, sets or removes `flags` on the messages of `mailbox` among
    /// `ids`, rewriting the file once for the whole set, and returns how
    /// many it found. An unknown id is skipped, and none found writes
    /// nothing.
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
    ) -> Result<usize> {
        let path = self.open_mbox(mailbox)?;
        let flags = flags_to_mbox(flags);
        let (index, mut cache) = self.index(&path, false)?;

        let found: Vec<String> = found(&index, ids, &path)
            .into_iter()
            .map(|entry| entry.id.clone())
            .collect();
        if found.is_empty() {
            return Ok(0);
        }

        let edit = |current: &MboxFlags| {
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
        };

        let ids: Vec<&str> = found.iter().map(String::as_str).collect();
        self.rewrite_flags(&path, &ids, edit, index, &mut cache)?;
        Ok(found.len())
    }

    /// Rewrites the flags of messages of the mbox at `path` to what `edit`
    /// makes of their current ones, in one locked rewrite of the file. An
    /// unknown id fails the whole set before any write.
    pub fn edit_flags(
        &self,
        path: &MboxFsPath,
        ids: &[&str],
        edit: impl Fn(&MboxFlags) -> MboxFlags,
    ) -> Result<()> {
        let (index, mut cache) = self.index(path, false)?;
        require_all(&index, ids, path)?;
        self.rewrite_flags(path, ids, edit, index, &mut cache)
    }

    /// Reads one message's raw RFC 5322 bytes, unquoted, from `mailbox`,
    /// without the fields the store owns: the flag fields, which the
    /// flags render, Thunderbird's status fields, and the `Content-Length`
    /// of the mboxcl variants.
    ///
    /// An index another client made stale is rebuilt and the read tried
    /// once more.
    pub fn get_message(&self, mailbox: &str, id: &str, seen: bool) -> Result<Vec<u8>> {
        let path = self.open_mbox(mailbox)?;
        let (index, _) = self.index(&path, false)?;

        let contents = match self.read(&path, find(&index, id, &path)?) {
            Err(err) if is_stale(&err) => {
                let (index, _) = self.index(&path, true)?;
                self.read(&path, find(&index, id, &path)?)?
            }
            result => result?,
        };
        let contents = strip_store_fields(&contents, self.format.has_content_length());

        if seen {
            let seen = Flag::from_iana(IanaFlag::Seen);
            self.store_flags(mailbox, &[id], &[seen], FlagOp::Add)?;
        }

        Ok(contents)
    }

    /// Appends `raw` to `mailbox` with `flags`, returning its id.
    pub fn add_message(&self, mailbox: &str, flags: &[Flag], raw: Vec<u8>) -> Result<String> {
        let path = self.open_mbox(mailbox)?;
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
        let appended = self.exec(MboxEntryAppend::new(path.clone(), vec![item], opts))?;
        let id = self.appended_ids(path, &appended)?.into_iter().next();
        id.ok_or_else(|| anyhow!("Cannot find the message appended to {path}"))
    }

    /// The ids the synced index of `path` gives the messages `appended` to
    /// it.
    ///
    /// An append knows the content id, not how many copies of the message
    /// the file already holds, which the numbered id needs.
    fn appended_ids(&self, path: &MboxFsPath, appended: &[MboxEntry]) -> Result<Vec<String>> {
        let (index, _) = self.index(path, false)?;
        appended
            .iter()
            .map(|appended| {
                index
                    .entries
                    .iter()
                    .find(|entry| entry.offset == appended.offset)
                    .map(|entry| entry.id.clone())
                    .ok_or_else(|| anyhow!("Cannot find the message appended to {path}"))
            })
            .collect()
    }

    /// Copies the messages of `from` among `ids` to the end of `to`,
    /// returning how many it found. An unknown id is skipped, and none
    /// found writes nothing.
    pub fn copy_messages(&self, from: &str, to: &str, ids: &[&str]) -> Result<usize> {
        Ok(self.copy_message_ids(from, to, ids)?.len())
    }

    /// Copies like [`Self::copy_messages`], returning the ids of the
    /// copies in `to`, each message found once.
    pub fn copy_message_ids(&self, from: &str, to: &str, ids: &[&str]) -> Result<Vec<String>> {
        let source = self.open_mbox(from)?;
        let target = self.open_mbox(to)?;
        let (index, _) = self.index(&source, false)?;
        let entries: Vec<MboxEntry> = found(&index, ids, &source).into_iter().cloned().collect();
        if entries.is_empty() {
            return Ok(Vec::new());
        }

        let opts = MboxEntryCopyOptions {
            format: self.format,
            lock: self.lock.clone(),
            scanner: self.scanner.clone(),
            chunk_size: self.chunk_size,
        };
        let copied = self.exec(MboxEntryCopy::new(source, target.clone(), entries, opts))?;
        self.appended_ids(&target, &copied)
    }

    /// Moves the messages of `from` among `ids` to the end of `to`: a
    /// copy, then a removal from the source. Returns how many it found,
    /// an unknown id being skipped and none found writing nothing.
    pub fn move_messages(&self, from: &str, to: &str, ids: &[&str]) -> Result<usize> {
        Ok(self.move_message_ids(from, to, ids)?.len())
    }

    /// Moves like [`Self::move_messages`], returning the ids of the
    /// messages in `to`, each message found once.
    pub fn move_message_ids(&self, from: &str, to: &str, ids: &[&str]) -> Result<Vec<String>> {
        let source = self.open_mbox(from)?;
        let target = self.open_mbox(to)?;
        let (index, mut cache) = self.index(&source, false)?;
        let entries: Vec<MboxEntry> = found(&index, ids, &source).into_iter().cloned().collect();
        if entries.is_empty() {
            return Ok(Vec::new());
        }

        let opts = MboxEntryMoveOptions {
            index: Some(index),
            format: self.format,
            lock: self.lock.clone(),
            scanner: self.scanner.clone(),
            chunk_size: self.chunk_size,
        };
        let output = self.exec(MboxEntryMove::new(
            source.clone(),
            target.clone(),
            entries,
            opts,
        ))?;

        cache.index = Some(output.index);
        cache.save(self.cache_dir.as_deref(), &source);
        self.appended_ids(&target, &output.entries)
    }

    /// Fails unless the mbox at `path` holds every id of `ids`, for the raw
    /// commands, which act on all of them or none.
    pub fn require_ids(&self, path: &MboxFsPath, ids: &[&str]) -> Result<()> {
        let (index, _) = self.index(path, false)?;
        require_all(&index, ids, path)
    }

    /// Permanently removes the messages of `mailbox` among `ids`,
    /// returning how many it found. An unknown id is skipped, and none
    /// found writes nothing.
    pub fn delete_messages(&self, mailbox: &str, ids: &[&str]) -> Result<usize> {
        let path = self.open_mbox(mailbox)?;
        let (index, mut cache) = self.index(&path, false)?;

        let edits: BTreeMap<String, MboxEntryUpdateEdit> = found(&index, ids, &path)
            .into_iter()
            .map(|entry| (entry.id.clone(), MboxEntryUpdateEdit::Remove))
            .collect();
        let count = edits.len();
        if count == 0 {
            return Ok(0);
        }

        self.update(&path, edits, index, &mut cache)?;
        Ok(count)
    }

    /// Creates the empty mbox `name` at the top of the store, or under
    /// `parent`, with its parent directories, returning its id, its name.
    ///
    /// In the plain layout a mailbox nests only under a directory: one
    /// under an mbox file is refused, naming `mbox.thunderbird`. In the
    /// Thunderbird layout a missing parent mbox is created empty, so the
    /// children listed under it can be renamed with it.
    pub fn create_mailbox(&self, name: &str, parent: Option<&str>) -> Result<String> {
        let name = match parent {
            Some(parent) => child_path(parent, '/', name),
            None => name.to_string(),
        };

        validate_mbox_name(&name, self.store.thunderbird)?;

        let components: Vec<&str> = name.split('/').collect();
        for depth in 1..components.len() {
            let parent = components[..depth].join("/");
            let path = self.store.resolve(&MboxPath::from(parent.as_str()));
            let path = Path::new(path.as_str());

            if self.store.thunderbird && !path.exists() {
                self.exec(MboxCreate::new(MboxFsPath::from(path)))?;
            } else if !self.store.thunderbird && path.is_file() {
                bail!(
                    "Cannot create mbox `{name}`: `{parent}` is an mbox file, which holds no mailbox unless `mbox.thunderbird` is set"
                );
            }
        }

        let path = self.store.resolve(&MboxPath::from(name.as_str()));
        self.exec(MboxCreate::new(path))?;
        Ok(name)
    }

    /// Renames the last level of the mbox `mailbox` to `name`, its parent
    /// kept, returning its new id. The target must not exist.
    pub fn rename_mailbox(&self, mailbox: &str, name: &str) -> Result<String> {
        let to = renamed_path(mailbox, '/', name);
        self.rename_mbox(mailbox, &to)?;
        Ok(to)
    }

    /// Renames the mbox `from` to `to`, which must not exist. In the
    /// Thunderbird layout the `.sbd` directory holding its children
    /// follows, so they keep their parent.
    ///
    /// The spool is refused either way, and so is a directory, which is
    /// no mbox.
    pub fn rename_mbox(&self, from: &str, to: &str) -> Result<()> {
        validate_mbox_name(from, self.store.thunderbird)?;
        validate_mbox_name(to, self.store.thunderbird)?;

        let from_path = self.resolve_mbox(from)?;
        let to_path = self.store.resolve(&MboxPath::from(to));
        self.refuse_spool(&from_path, "rename", from)?;
        self.refuse_spool(&to_path, "rename a mailbox to", to)?;
        refuse_dir(&from_path, "rename", from)?;
        let from_sbd = from_path.with_suffix(SBD);
        let to_sbd = to_path.with_suffix(SBD);
        let subtree = self.store.thunderbird && Path::new(from_sbd.as_str()).is_dir();

        if subtree && fs::symlink_metadata(to_sbd.as_str()).is_ok() {
            bail!("Cannot rename mbox `{from}` to `{to}`: {to_sbd} already exists");
        }

        self.exec(MboxRename::new(from_path.clone(), to_path.clone()))?;

        if subtree && let Err(err) = fs::rename(from_sbd.as_str(), to_sbd.as_str()) {
            // Put the file back, so the mbox and its children stay together.
            self.exec(MboxRename::new(to_path, from_path))
                .context("Cannot move the renamed mbox back")?;
            return Err(err).with_context(|| format!("Cannot rename {from_sbd} to {to_sbd}"));
        }

        Ok(())
    }

    /// Deletes the mbox `name` and every message in it, under lock so no
    /// delivery lands in it meanwhile. In the Thunderbird layout an mbox
    /// holding others is refused, naming them, and its emptied `.sbd`
    /// directory goes with it.
    ///
    /// The spool is refused, the mail system delivering to it, and so is
    /// a directory, which is no mbox.
    pub fn delete_mailbox(&self, name: &str) -> Result<()> {
        validate_mbox_name(name, self.store.thunderbird)?;
        let path = self.resolve_mbox(name)?;
        self.refuse_spool(&path, "delete", name)?;
        refuse_dir(&path, "delete", name)?;

        if self.store.thunderbird {
            let prefix = format!("{name}/");
            let children: Vec<String> = self
                .list_mboxes()?
                .into_iter()
                .map(|mbox| mbox.name.to_string())
                .filter(|child| child.starts_with(&prefix))
                .collect();

            if !children.is_empty() {
                let children = children.join("`, `");
                bail!("Cannot delete mbox `{name}`: it holds `{children}`; delete them first");
            }
        }

        self.exec(MboxDelete::new(path.clone(), self.lock.clone()))?;

        let sbd = path.with_suffix(SBD);
        if self.store.thunderbird
            && let Err(err) = fs::remove_dir(sbd.as_str())
            && err.kind() != io::ErrorKind::NotFound
        {
            warn!("cannot remove {sbd}, left in place: {err}");
        }

        Ok(())
    }

    /// Fails when `path` is the spool `mbox.inbox` names: the mail system
    /// delivers to it, and IMAP refuses to delete or rename `INBOX`
    /// alike.
    fn refuse_spool(&self, path: &MboxFsPath, verb: &str, name: &str) -> Result<()> {
        let Some(spool) = &self.store.inbox else {
            return Ok(());
        };

        let same = spool == path
            || matches!(
                (fs::canonicalize(spool.as_str()), fs::canonicalize(path.as_str())),
                (Ok(spool), Ok(path)) if spool == path
            );
        if same {
            bail!(
                "Cannot {verb} `{name}`: it is the spool `mbox.inbox` names, which the mail system delivers to"
            );
        }

        Ok(())
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

    /// Rewrites the flags of the messages of `index` among `ids` to what
    /// `edit` makes of their current ones, under lock, and caches the
    /// rewritten index.
    fn rewrite_flags(
        &self,
        path: &MboxFsPath,
        ids: &[&str],
        edit: impl Fn(&MboxFlags) -> MboxFlags,
        index: MboxIndex,
        cache: &mut MboxCache,
    ) -> Result<()> {
        let edits = ids
            .iter()
            .filter_map(|id| index.get(id))
            .map(|entry| {
                let flags = MboxEntryUpdateEdit::Flags(edit(&entry.flags));
                (entry.id.clone(), flags)
            })
            .collect();

        self.update(path, edits, index, cache)
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
        let output = self.exec(MboxEntryUpdate::new(path.clone(), edits, opts))?;

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
    ///
    /// A read finding the index stale, another client having rewritten the
    /// file meanwhile, rebuilds it and starts over once.
    fn envelopes(
        &self,
        path: &MboxFsPath,
        contents: bool,
    ) -> Result<Vec<(Envelope, Option<Vec<u8>>)>> {
        match self.envelopes_once(path, contents, false) {
            Err(err) if err.downcast_ref().is_some_and(is_stale) => {
                self.envelopes_once(path, contents, true)
            }
            result => result,
        }
    }

    /// One pass of [`Self::envelopes`], over the index rebuilt from
    /// scratch when `rebuild`.
    fn envelopes_once(
        &self,
        path: &MboxFsPath,
        contents: bool,
        rebuild: bool,
    ) -> Result<Vec<(Envelope, Option<Vec<u8>>)>> {
        let (index, mut cache) = self.index(path, rebuild)?;
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

/// Fails when `path` is a directory, which holds mailboxes but is none.
fn refuse_dir(path: &MboxFsPath, verb: &str, name: &str) -> Result<()> {
    if Path::new(path.as_str()).is_dir() {
        bail!("Cannot {verb} `{name}`: it is a directory holding mailboxes, not an mbox");
    }
    Ok(())
}

/// Whether a read failed on an entry another client moved.
fn is_stale(err: &MboxClientError) -> bool {
    matches!(err, MboxClientError::EntryGet(MboxEntryGetError::Stale(_)))
}

/// Finds the entry `id` names in `index`.
fn find<'a>(index: &'a MboxIndex, id: &str, path: &MboxFsPath) -> Result<&'a MboxEntry> {
    index
        .get(id)
        .filter(|entry| !entry.pseudo)
        .ok_or_else(|| message_not_found(format!("Message `{id}` not found in mbox {path}")))
}

/// Fails with `message-not-found` unless `index` holds every id of
/// `ids`, naming each unknown one, before any write.
fn require_all(index: &MboxIndex, ids: &[&str], path: &MboxFsPath) -> Result<()> {
    let mut missing: Vec<&str> = Vec::new();
    for id in ids {
        if find(index, id, path).is_err() && !missing.contains(id) {
            missing.push(id);
        }
    }

    if !missing.is_empty() {
        let missing = missing.join("`, `");
        return Err(message_not_found(format!(
            "No message changed: `{missing}` not found in mbox {path}"
        )));
    }

    Ok(())
}

/// The entries of `index` among `ids`, each once, in the order given. An
/// unknown id is skipped with a warning.
fn found<'a>(index: &'a MboxIndex, ids: &[&str], path: &MboxFsPath) -> Vec<&'a MboxEntry> {
    let mut seen = BTreeSet::new();
    let mut found = Vec::with_capacity(ids.len());

    for id in ids {
        match index.get(id) {
            Some(entry) if !entry.pseudo => {
                if seen.insert(entry.id.as_str()) {
                    found.push(entry);
                }
            }
            _ => warn!("message `{id}` not found in mbox {path}, skipping it"),
        }
    }

    found
}

/// Leaves out of a stored message the header fields the store owns: the
/// flag fields, and `Content-Length` when `content_length` (the mboxcl
/// variants, whose append writes its own).
fn strip_store_fields(contents: &[u8], content_length: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(contents.len());
    let mut dropping = false;
    let mut lines = contents.split_inclusive(|b| *b == b'\n');

    for line in lines.by_ref() {
        if line == b"\n" || line == b"\r\n" {
            out.extend_from_slice(line);
            break;
        }

        if !matches!(line.first(), Some(b' ' | b'\t')) {
            let name = line.split(|b| *b == b':').next().unwrap_or_default();
            dropping = STORE_FIELDS
                .iter()
                .any(|field| name.eq_ignore_ascii_case(field.as_bytes()))
                || (content_length && name.eq_ignore_ascii_case(b"Content-Length"));
        }

        if !dropping {
            out.extend_from_slice(line);
        }
    }

    for line in lines {
        out.extend_from_slice(line);
    }

    out
}

/// Parses a message into a shared [`Envelope`], its id and flags left
/// empty for the caller to fill from the index.
fn envelope_from(contents: &[u8]) -> Envelope {
    envelope::from_raw(contents, true)
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
    use tempfile::TempDir;

    use super::*;
    use crate::config::{MboxConfig, MboxFormatConfig, MboxLockConfig};

    const MESSAGE: &[u8] = b"From: a@x.org\nSubject: hi\nMessage-ID: <1@x>\n\nbody\n";

    /// A client over a fresh root, with no cache and no dotlock.
    fn client(format: MboxFormatConfig, thunderbird: bool) -> (TempDir, MboxClient) {
        let root = tempfile::tempdir().unwrap();
        let mut client = MboxClient::new(MboxConfig {
            root: root.path().to_path_buf(),
            inbox: None,
            format,
            thunderbird,
            lock: MboxLockConfig {
                dotlock: false,
                fcntl: true,
                timeout: Some(1),
            },
        });
        client.cache_dir = None;
        (root, client)
    }

    /// The names of the mailboxes of `client`.
    fn names(client: &MboxClient) -> Vec<String> {
        let mailboxes = client.list_mailboxes(false).unwrap();
        mailboxes.into_iter().map(|mailbox| mailbox.name).collect()
    }

    #[test]
    fn unknown_and_repeated_ids_are_neither_acted_on_nor_counted() {
        let (_root, client) = client(MboxFormatConfig::Mboxrd, false);
        client.create_mailbox("A", None).unwrap();
        client.create_mailbox("B", None).unwrap();
        let id = client.add_message("A", &[], MESSAGE.to_vec()).unwrap();
        let flagged = [Flag::from_iana(IanaFlag::Flagged)];

        let count = client.store_flags("A", &["nope"], &flagged, FlagOp::Add);
        assert_eq!(count.unwrap(), 0);
        let count = client.store_flags("A", &[&id, "nope", &id], &flagged, FlagOp::Add);
        assert_eq!(count.unwrap(), 1);

        assert_eq!(client.copy_messages("A", "B", &["nope"]).unwrap(), 0);
        assert_eq!(client.copy_messages("A", "B", &[&id, &id]).unwrap(), 1);
        assert_eq!(
            client.list_envelopes("B", None, None, false).unwrap().len(),
            1
        );

        assert_eq!(client.move_messages("A", "B", &["nope"]).unwrap(), 0);
        assert_eq!(client.delete_messages("A", &["nope"]).unwrap(), 0);
        assert_eq!(
            client.list_envelopes("A", None, None, false).unwrap().len(),
            1
        );

        assert_eq!(client.delete_messages("B", &[&id, "nope", &id]).unwrap(), 1);
        assert!(
            client
                .list_envelopes("B", None, None, false)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn a_read_leaves_out_the_fields_the_store_writes() {
        for format in [MboxFormatConfig::Mboxcl, MboxFormatConfig::Mboxcl2] {
            let (_root, client) = client(format, false);
            client.create_mailbox("A", None).unwrap();
            let flags = [
                Flag::from_iana(IanaFlag::Seen),
                Flag::from_iana(IanaFlag::Flagged),
                Flag::from_raw("work"),
            ];
            let id = client.add_message("A", &flags, MESSAGE.to_vec()).unwrap();

            assert_eq!(client.get_message("A", &id, false).unwrap(), MESSAGE);
        }
    }

    #[test]
    fn only_the_store_fields_are_stripped() {
        let stored = b"Status: RO\nSubject: x\nX-Keywords: a\n  b\nContent-Length: 5\nLines: 1\n\nStatus: body\n";

        assert_eq!(
            strip_store_fields(stored, true),
            b"Subject: x\nLines: 1\n\nStatus: body\n"
        );
        assert_eq!(
            strip_store_fields(stored, false),
            b"Subject: x\nContent-Length: 5\nLines: 1\n\nStatus: body\n"
        );
    }

    #[test]
    fn a_thunderbird_rename_moves_the_children() {
        let (root, client) = client(MboxFormatConfig::Mboxrd, true);
        client.create_mailbox("P", None).unwrap();
        client.create_mailbox("Kid", Some("P")).unwrap();
        client.add_message("P/Kid", &[], MESSAGE.to_vec()).unwrap();

        assert_eq!(client.rename_mailbox("P", "Q").unwrap(), "Q");

        assert_eq!(names(&client), ["Q", "Q/Kid"]);
        assert!(!root.path().join("P.sbd").exists());
        assert_eq!(
            client
                .list_envelopes("Q/Kid", None, None, false)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn a_thunderbird_rename_keeps_a_target_subtree() {
        let (root, client) = client(MboxFormatConfig::Mboxrd, true);
        client.create_mailbox("P", None).unwrap();
        client.create_mailbox("Kid", Some("P")).unwrap();
        fs::create_dir(root.path().join("Q.sbd")).unwrap();

        let err = client.rename_mailbox("P", "Q").unwrap_err().to_string();
        assert!(err.contains("Q.sbd already exists"), "{err}");
        assert_eq!(names(&client), ["P", "P/Kid"]);
    }

    #[test]
    fn a_thunderbird_delete_refuses_a_parent() {
        let (root, client) = client(MboxFormatConfig::Mboxrd, true);
        client.create_mailbox("P", None).unwrap();
        client.create_mailbox("Kid", Some("P")).unwrap();

        let err = client.delete_mailbox("P").unwrap_err().to_string();
        assert!(err.contains("it holds `P/Kid`"), "{err}");
        assert_eq!(names(&client), ["P", "P/Kid"]);

        client.delete_mailbox("P/Kid").unwrap();
        client.delete_mailbox("P").unwrap();
        assert!(names(&client).is_empty());
        assert!(!root.path().join("P.sbd").exists());
    }

    #[test]
    fn a_plain_mbox_file_holds_no_mailbox() {
        let (root, client) = client(MboxFormatConfig::Mboxrd, false);
        client.create_mailbox("A", None).unwrap();

        let err = client.create_mailbox("Sub", Some("A")).unwrap_err();
        assert!(err.to_string().contains("mbox.thunderbird"), "{err}");
        assert!(root.path().join("A").is_file());

        assert_eq!(
            client.create_mailbox("rust", Some("lists")).unwrap(),
            "lists/rust"
        );
    }

    #[test]
    fn a_missing_root_is_no_empty_store() {
        let (root, client) = client(MboxFormatConfig::Mboxrd, false);
        drop(root);

        let err = client.list_mailboxes(false).unwrap_err().to_string();
        assert!(err.contains("does not exist"), "{err}");
    }

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

    #[test]
    fn the_spool_is_neither_deleted_nor_renamed() {
        let (root, mut client) = client(MboxFormatConfig::Mboxrd, false);
        let spool = root.path().join("spool.mbox");
        fs::write(&spool, b"").unwrap();
        client.store.inbox = Some(MboxFsPath::from(spool.as_path()));
        client.add_message("INBOX", &[], MESSAGE.to_vec()).unwrap();
        client.create_mailbox("A", None).unwrap();

        for name in ["INBOX", "inbox"] {
            let err = client.delete_mailbox(name).unwrap_err().to_string();
            assert!(err.contains("`mbox.inbox`"), "{err}");
            let err = client.rename_mbox(name, "Moved").unwrap_err().to_string();
            assert!(err.contains("`mbox.inbox`"), "{err}");
        }
        assert!(client.rename_mailbox("A", "INBOX").is_err());
        assert!(client.delete_mailbox(spool.to_str().unwrap()).is_err());

        let inbox = client.list_envelopes("INBOX", None, None, false).unwrap();
        assert_eq!(inbox.len(), 1);
        assert_eq!(names(&client), ["A", "INBOX"]);
    }

    #[test]
    fn a_listing_rebuilds_a_stale_index_once() {
        let (_root, mut client) = client(MboxFormatConfig::Mboxrd, false);
        let cache_dir = tempfile::tempdir().unwrap();
        client.cache_dir = Some(cache_dir.path().to_path_buf());
        client.create_mailbox("A", None).unwrap();
        client.add_message("A", &[], MESSAGE.to_vec()).unwrap();
        let other = b"From: b@x.org\nSubject: two\n\nbody two\n";
        client.add_message("A", &[], other.to_vec()).unwrap();

        // NOTE: a cached index whose file did not change on disk is reused
        // as is, so a wrong entry stands for one another client moved.
        let path = client.resolve_mbox("A").unwrap();
        let mut cache = MboxCache::load(client.cache_dir.as_deref(), &path);
        let index = cache.index.as_mut().unwrap();
        index.entries[0].len -= 1;
        cache.save(client.cache_dir.as_deref(), &path);

        let query = "body body".parse::<SearchEmailsQuery>().unwrap();
        let hits = client.search_envelopes("A", Some(&query), None, None, false);
        assert_eq!(hits.unwrap().len(), 2);
    }

    #[test]
    fn copies_report_the_ids_they_get_in_the_target() {
        let (_root, client) = client(MboxFormatConfig::Mboxrd, false);
        client.create_mailbox("A", None).unwrap();
        client.create_mailbox("B", None).unwrap();
        let id = client.add_message("A", &[], MESSAGE.to_vec()).unwrap();
        client.add_message("B", &[], MESSAGE.to_vec()).unwrap();

        let ids = client.copy_message_ids("A", "B", &[&id, &id]).unwrap();
        assert_eq!(ids, [format!("{id}-2")]);
        let ids = client.move_message_ids("A", "B", &[&id, &id]).unwrap();
        assert_eq!(ids, [format!("{id}-3")]);
    }

    #[test]
    fn names_a_listing_hides_and_directories_are_refused() {
        let (root, client) = client(MboxFormatConfig::Mboxrd, false);
        for name in [".hidden", "Y.msf", "Z.lock"] {
            assert!(client.create_mailbox(name, None).is_err(), "{name}");
        }

        client.create_mailbox("Leaf", Some("dir")).unwrap();
        let err = client.delete_mailbox("dir").unwrap_err().to_string();
        assert!(err.contains("directory"), "{err}");
        let err = client.rename_mbox("dir", "newdir").unwrap_err().to_string();
        assert!(err.contains("directory"), "{err}");
        assert!(root.path().join("dir/Leaf").is_file());
    }

    #[test]
    fn a_thunderbird_child_gets_its_parent() {
        let (root, client) = client(MboxFormatConfig::Mboxrd, true);
        assert!(client.create_mailbox("X.sbd", None).is_err());

        client.create_mailbox("Ghost/Kid", None).unwrap();
        assert!(root.path().join("Ghost").is_file());
        assert_eq!(names(&client), ["Ghost", "Ghost/Kid"]);
        client.rename_mailbox("Ghost", "G2").unwrap();
        assert_eq!(names(&client), ["G2", "G2/Kid"]);
    }

    #[test]
    fn a_read_leaves_out_the_thunderbird_fields() {
        let (root, client) = client(MboxFormatConfig::Mboxrd, false);
        let stored = b"From - Mon Jan  1 00:00:00 2024\nX-Mozilla-Status: 0001\nX-Mozilla-Status2: 00000000\nX-Mozilla-Keys: $label1\nFrom: a@x.org\nSubject: hi\nMessage-ID: <1@x>\n\nbody\n";
        fs::write(root.path().join("T"), stored).unwrap();
        let id = client.list_envelopes("T", None, None, false).unwrap()[0]
            .id
            .clone();

        assert_eq!(client.get_message("T", &id, false).unwrap(), MESSAGE);

        let seen = [Flag::from_iana(IanaFlag::Seen)];
        client
            .store_flags("T", &[&id], &seen, FlagOp::Remove)
            .unwrap();
        let stored = fs::read_to_string(root.path().join("T")).unwrap();
        assert!(stored.contains("X-Mozilla-Status: 0000\n"), "{stored}");
    }
}
