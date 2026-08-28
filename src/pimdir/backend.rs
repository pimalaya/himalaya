//! pimdir adapter for the shared cross-protocol client.
//!
//! Reads project [`io_pimdir`]'s client read API (`list_collections_by_account`,
//! `list_items`, `get_item`, `count_items`) plus the blob store, building
//! envelopes from the stored `v: 1` meta (pimdir SPEC Annex A) with no body reads.
//! An item whose body is not local (`level < Full`) still lists; `get_message`
//! reports "body not fetched" rather than an error — the cue to sync.
//!
//! Writes enqueue [`PimdirAction`]s for the store's owner to apply (pimdir SPEC
//! §15.1). Himalaya is a producer, not the owner: it never mutates the index and
//! never collects, so a staged flag change cannot race a sync mid-hydration. The
//! owner drains the queue on its next run and derives the push from there.
//!
//! A mailbox is its collection id, verbatim: the sync binds a source's
//! collections under a namespace, so the mailbox a server calls `INBOX` is
//! `imap/INBOX` here and is addressed by that. The id is opaque to the store,
//! which parses it nowhere, so shortening it would be a guess at the sync's
//! convention rather than a lookup. [`PimdirClient::hub_id`] checks one against
//! the account's collections, and `[mailbox.alias]` is how a user avoids typing
//! it, as for the JMAP ids it resembles.

use anyhow::{Result, anyhow, bail};
use chrono::{DateTime, SecondsFormat, Utc};
use io_pimdir::{
    PimdirCollection, PimdirItem, PimdirPendingAction,
    codec::PimdirAction,
    conventions::{self, PimdirDerivation},
};
use io_replica::{
    collection::ReplicaCollectionId,
    placement::{ReplicaFlags, ReplicaLevel, ReplicaLinkId},
};
use log::warn;
use serde::Deserialize;

use crate::{
    email::{
        address::Address,
        envelope::Envelope,
        flag::{Flag, FlagOp, IanaFlag},
        mailbox::Mailbox,
        search::{eval, query::SearchEmailsQuery},
    },
    pimdir::client::PimdirClient,
};

/// The mail media type a pimdir collection carries to be a mailbox.
const MAIL_KIND: &str = "message/rfc822";

/// How many items to pull per keyset page when scanning a whole collection.
const SCAN_BATCH: usize = 500;

/// Whether a collection's declared kind makes it a mailbox.
///
/// A kind-less collection counts: a sync that created one before kinds were
/// declared left the column empty, and refusing those would hide the mailboxes
/// of every store written back then.
pub(crate) fn is_mail(kind: &str) -> bool {
    kind.is_empty() || kind == MAIL_KIND
}

/// A reader's view of the `message/rfc822` meta (pimdir SPEC Annex A).
#[derive(Default, Deserialize)]
struct MetaView {
    #[serde(default)]
    message_id: Option<String>,
    #[serde(default)]
    in_reply_to: Vec<String>,
    #[serde(default)]
    subject: String,
    #[serde(default)]
    from: Option<String>,
    #[serde(default)]
    to: Option<String>,
    #[serde(default)]
    date: Option<String>,
    #[serde(default)]
    size: Option<u64>,
}

impl PimdirClient {
    // --- Mailbox naming --------------------------------------------------

    /// The collection a user-typed mailbox addresses, which is the mailbox
    /// itself: a pimdir mailbox is its collection id, verbatim.
    ///
    /// The id is checked against the account's mail collections rather than
    /// passed through. An id nothing was written under reads as a mailbox that
    /// exists and is empty, so an unknown one is refused here, naming what the
    /// account does hold.
    pub(crate) fn hub_id(&self, mailbox: &str) -> Result<String> {
        let mut ids: Vec<String> = self
            .mail_collections()?
            .into_iter()
            .map(|collection| collection.id)
            .collect();

        if ids.iter().any(|id| id == mailbox) {
            return Ok(mailbox.to_string());
        }

        ids.sort();

        bail!(
            "Mailbox `{mailbox}` not found in the pimdir store, which holds: {}",
            ids.join(", "),
        )
    }

    /// The account's mail collections, in store order.
    fn mail_collections(&self) -> Result<Vec<PimdirCollection>> {
        Ok(self
            .store
            .list_collections_by_account(self.account.as_deref())
            .map_err(|err| anyhow!("List pimdir collections: {err}"))?
            .into_iter()
            .filter(|collection| is_mail(&collection.kind))
            .collect())
    }

    // --- Reads -----------------------------------------------------------

    /// Lists the account's mail collections, each addressed by its collection
    /// id and named by the collection row's own name, sorted by id.
    /// `with_counts` fills `total` with the live item count.
    pub fn list_mailboxes(&mut self, with_counts: bool) -> Result<Vec<Mailbox>> {
        let mut mailboxes = Vec::new();
        for collection in self.mail_collections()? {
            let total = if with_counts {
                Some(
                    self.store
                        .count_items(&collection.id)
                        .map_err(|err| anyhow!("Count items in `{}`: {err}", collection.id))?,
                )
            } else {
                None
            };
            mailboxes.push(Mailbox {
                id: collection.id,
                name: collection.name,
                total,
                unread: None,
            });
        }
        mailboxes.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(mailboxes)
    }

    /// Lists envelopes from `mailbox`, built from the stored meta (no body
    /// reads), sorted by `Date:` descending then paginated.
    pub fn list_envelopes(
        &mut self,
        mailbox: &str,
        page: Option<u32>,
        page_size: Option<u32>,
        _with_attachment: bool,
    ) -> Result<Vec<Envelope>> {
        let collection = self.hub_id(mailbox)?;
        let mut envelopes: Vec<Envelope> = self
            .scan_items(&collection)?
            .iter()
            .map(envelope_from_item)
            .collect();
        envelopes.sort_by_key(|envelope| std::cmp::Reverse(envelope.date));
        Ok(paginate(envelopes, page, page_size))
    }

    /// Searches envelopes in `mailbox`: builds them from meta, applies the
    /// shared filter/sort/paginate. Body clauses cannot match on an item whose
    /// body is not local (no bytes to scan); header/flag clauses always do.
    pub fn search_envelopes(
        &mut self,
        mailbox: &str,
        query: Option<&SearchEmailsQuery>,
        page: Option<u32>,
        page_size: Option<u32>,
        _with_attachment: bool,
    ) -> Result<Vec<Envelope>> {
        let collection = self.hub_id(mailbox)?;
        let filter = query.and_then(|q| q.filter.as_ref());
        let mut hits: Vec<Envelope> = self
            .scan_items(&collection)?
            .iter()
            .map(envelope_from_item)
            .filter(|envelope| match filter {
                Some(filter) => eval::matches_filter(envelope, &[], filter),
                None => true,
            })
            .collect();
        eval::sort_envelopes(&mut hits, query.and_then(|q| q.sort.as_deref()));
        Ok(paginate(hits, page, page_size))
    }

    /// How many messages the mailbox has queued for creation and not yet
    /// synced (pimdir SPEC §15.4).
    ///
    /// A queued create has no public id until the store's owner applies it, so
    /// it is not an envelope and does not list; the count is what a listing
    /// reports instead, so a saved message that is not in the list reads as
    /// queued rather than as lost.
    pub fn queued_messages(&mut self, mailbox: &str) -> Result<usize> {
        let collection = self.hub_id(mailbox)?;
        self.store
            .count_pending_creates(&collection)
            .map_err(|err| anyhow!("Count queued messages in `{mailbox}`: {err}"))
    }

    /// The mailbox's queued creations, rendered as mail.
    ///
    /// The operator CLI is kind-agnostic and prints ids, hashes and flags; this
    /// client holds the conventions and the blobs, so it reads the sender,
    /// subject and date out of the action's own `v: 1` summary.
    pub fn queued_envelopes(&mut self, mailbox: &str) -> Result<Vec<PimdirQueued>> {
        let collection = self.hub_id(mailbox)?;
        let queued = self
            .store
            .pending_creates(&collection)
            .map_err(|err| anyhow!("List queued messages in `{mailbox}`: {err}"))?;

        Ok(queued.iter().filter_map(queued_from_action).collect())
    }

    /// Reads one message's raw bytes from its content-addressed blob. Fails with
    /// a clear "body not fetched" when the item is not hydrated to `Full` (no
    /// local body) — the client's cue to sync rather than a data-loss error.
    pub fn get_message(&mut self, mailbox: &str, id: &str, seen: bool) -> Result<Vec<u8>> {
        let collection = self.hub_id(mailbox)?;
        let Some(item) = self.get(&collection, id)? else {
            bail!("Message `{id}` not found in `{mailbox}`");
        };
        let Some(hash) = item.object else {
            bail!(
                "Message `{id}` in `{mailbox}` is not downloaded yet (body not fetched); \
                 run a sync to hydrate it"
            );
        };
        let bytes = self
            .blobs
            .get(&hash)?
            .ok_or_else(|| anyhow!("Body blob missing for `{id}` in `{mailbox}`"))?;

        // `--seen` stages a flag change; a read stays non-mutating by default,
        // and a store that refuses the staging must not fail the read.
        if seen {
            let seen_flag = Flag::from_iana(IanaFlag::Seen);
            if let Err(err) = self.store_flags(mailbox, &[id], &[seen_flag], FlagOp::Add) {
                warn!("could not stage \\Seen on `{id}` in `{mailbox}`: {err:#}");
            }
        }

        Ok(bytes)
    }

    // --- Writes (queued actions the store's owner applies) ---------------

    /// Adds, sets, or removes `flags` on an id set, staged as `SetFlags`.
    ///
    /// The action carries the whole replacement set, never a delta, so the owner
    /// applying it twice lands the same state.
    pub fn store_flags(
        &mut self,
        mailbox: &str,
        ids: &[&str],
        flags: &[Flag],
        op: FlagOp,
    ) -> Result<()> {
        let collection = self.hub_id(mailbox)?;
        let mut producer = self.producer()?;
        let now = stamp();

        for id in ids {
            let seq = self.seq(&collection, id)?;
            let current = self
                .get(&collection, id)?
                .map(|item| item.flags)
                .unwrap_or(ReplicaFlags::Unknown);
            let action = PimdirAction::SetFlags {
                seq,
                flags: apply_flag_op(&current, flags, op),
            };
            producer
                .enqueue(&collection, &action, None, &now)
                .map_err(|err| anyhow!("Stage flags on `{id}` in `{mailbox}`: {err}"))?;
        }
        Ok(())
    }

    /// Appends `raw` to `mailbox` as a locally-authored item, staged as `Add`
    /// (the next sync uploads it). Returns the link id it is stored under.
    ///
    /// The body lands in the blob store first and durably, then the action
    /// referencing it is enqueued: the queue row pins the object, so nothing
    /// collects a body between the two.
    ///
    /// The link id is the bare `Message-ID` [`derive`] gives, and a staged add
    /// whose link id the collection already holds parks (pimdir SPEC §15.3):
    /// it neither deduplicates against the stored copy nor mints a key of its
    /// own. The store answers the two producers differently on purpose.
    /// Minting is its answer to what a source hands over, a replica owing the
    /// collection what the collection holds, so a mailbox holding one
    /// `Message-ID` twice keeps both items (SPEC §9). Parking is its answer to
    /// a producer authoring a message the collection already has, which named
    /// a key it does not own and is told so rather than having its message
    /// filed under a key it never asked for.
    pub fn add_message(&mut self, mailbox: &str, flags: &[Flag], raw: Vec<u8>) -> Result<String> {
        let collection = self.hub_id(mailbox)?;
        let derived = derive(&raw)?;

        let hash = self.blobs.hash(&raw);
        let writer = self.blobs.writer()?;
        let size = write_blob(writer, &raw, &hash)?;

        let mut producer = self.producer()?;
        let action = PimdirAction::Add {
            link_id: Some(derived.link_id.clone()),
            flags: to_replica_flags(flags),
            object: Some(hash),
            meta: Some(derived.meta),
            handle: None,
        };
        producer
            .enqueue(&collection, &action, Some(size), &stamp())
            .map_err(|err| anyhow!("Stage add in `{mailbox}`: {err}"))?;

        Ok(derived.link_id.0)
    }

    /// Copies each id from `from` to `to`, staged as `Copy` (a server-side copy
    /// on the next sync, no body re-upload).
    pub fn copy_messages(&mut self, from: &str, to: &str, ids: &[&str]) -> Result<usize> {
        self.refile(from, to, ids, |seq, target| PimdirAction::Copy {
            seq,
            to: target,
        })
    }

    /// Moves each id from `from` to `to`, staged as `Move` (one server-side move
    /// on the next sync).
    pub fn move_messages(&mut self, from: &str, to: &str, ids: &[&str]) -> Result<usize> {
        self.refile(from, to, ids, |seq, target| PimdirAction::Move {
            seq,
            to: target,
        })
    }

    /// Deletes each id from `mailbox`, staged as `Remove` (the next sync pushes
    /// it as the backend's own disposal).
    pub fn delete_messages(&mut self, mailbox: &str, ids: &[&str]) -> Result<()> {
        let collection = self.hub_id(mailbox)?;
        let mut producer = self.producer()?;
        let now = stamp();

        for id in ids {
            let seq = self.seq(&collection, id)?;
            producer
                .enqueue(&collection, &PimdirAction::Remove { seq }, None, &now)
                .map_err(|err| anyhow!("Stage delete of `{id}` in `{mailbox}`: {err}"))?;
        }
        Ok(())
    }

    // --- Internals -------------------------------------------------------

    /// Stages one refiling action per id, `build` deciding whether the source
    /// copy stays.
    fn refile(
        &mut self,
        from: &str,
        to: &str,
        ids: &[&str],
        build: fn(i64, ReplicaCollectionId) -> PimdirAction,
    ) -> Result<usize> {
        let source = self.hub_id(from)?;
        let target = ReplicaCollectionId(self.hub_id(to)?);
        let mut producer = self.producer()?;
        let now = stamp();

        for id in ids {
            let seq = self.seq(&source, id)?;
            producer
                .enqueue(&source, &build(seq, target.clone()), None, &now)
                .map_err(|err| anyhow!("Stage refile of `{id}` from `{from}` to `{to}`: {err}"))?;
        }
        Ok(ids.len())
    }

    /// The item behind a public id, or `None` when the collection holds none.
    fn get(&self, collection: &str, id: &str) -> Result<Option<PimdirItem>> {
        self.store
            .get_item(collection, parse_id(id)?)
            .map_err(|err| anyhow!("Read `{id}` in `{collection}`: {err}"))
    }

    /// The public id an action addresses, checked against the collection so a
    /// stale id is refused here rather than parked by the owner much later.
    fn seq(&self, collection: &str, id: &str) -> Result<i64> {
        match self.get(collection, id)? {
            Some(item) => Ok(item.seq),
            None => bail!("Message `{id}` not found in `{}`", collection),
        }
    }

    /// Pulls every live item of a collection by keyset paging (the read API is
    /// paginated; the shared list/search commands sort and paginate in memory,
    /// as the file backends do).
    fn scan_items(&self, collection: &str) -> Result<Vec<PimdirItem>> {
        let mut all = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let page = self
                .store
                .list_items(collection, cursor.as_deref(), SCAN_BATCH)
                .map_err(|err| anyhow!("List items in `{collection}`: {err}"))?;
            let n = page.len();
            if let Some(last) = page.last() {
                cursor = Some(last.link_id.0.clone());
            }
            all.extend(page);
            if n < SCAN_BATCH {
                break;
            }
        }
        Ok(all)
    }
}

/// Builds a shared [`Envelope`] from a stored item's meta (no body read). Flags
/// come from the item; the display fields from the `v: 1` meta.
fn envelope_from_item(item: &PimdirItem) -> Envelope {
    let view: MetaView = item
        .meta
        .as_ref()
        .and_then(|meta| serde_json::from_str(&meta.0).ok())
        .unwrap_or_default();

    let flags = item
        .flags
        .known()
        .map(|flags| {
            flags
                .iter()
                .map(|raw| Flag::from_raw(raw.clone()))
                .collect()
        })
        .unwrap_or_default();
    let from = view
        .from
        .map(|email| vec![Address { name: None, email }])
        .unwrap_or_default();
    let to = view
        .to
        .map(|email| vec![Address { name: None, email }])
        .unwrap_or_default();
    let date = view
        .date
        .as_deref()
        .and_then(|d| DateTime::parse_from_rfc3339(d).ok());

    Envelope {
        // The public id: a short store-global integer, not the long link id.
        id: item.seq.to_string(),
        message_id: view.message_id,
        in_reply_to: view.in_reply_to,
        flags,
        subject: view.subject,
        from,
        to,
        date,
        size: view.size.unwrap_or(0),
        has_attachment: None,
    }
}

/// One queued creation, as `pimdir queue list` shows it: the row an operator
/// acts on, plus the mail the action carries.
#[derive(Clone, Debug)]
pub struct PimdirQueued {
    /// The queue row id, which `pimdir queue cancel` takes.
    pub id: i64,
    /// The RFC 3339 instant the row was enqueued, stamped by the store.
    pub created_at: String,
    /// The process that staged it.
    pub producer: String,
    /// The message the action carries, built from its `v: 1` summary. It has
    /// no `id`: a create has none until the owner applies it.
    pub envelope: Envelope,
}

/// Builds a queued row from a pending `Add`, skipping any other kind.
///
/// The envelope keeps an empty id on purpose. A create has no public id yet,
/// and putting the queue row id there would be an identifier from another
/// space in the field every command reads back.
fn queued_from_action(queued: &PimdirPendingAction) -> Option<PimdirQueued> {
    let PimdirAction::Add { flags, meta, .. } = &queued.action else {
        return None;
    };

    let item = PimdirItem {
        seq: 0,
        link_id: ReplicaLinkId(String::new()),
        flags: flags.clone(),
        meta: meta.clone(),
        sort_key: String::new(),
        object: None,
        level: ReplicaLevel::Meta,
        retention: None,
    };
    let mut envelope = envelope_from_item(&item);
    envelope.id = String::new();

    Some(PimdirQueued {
        id: queued.id,
        created_at: queued.created_at.clone(),
        producer: queued.producer.clone(),
        envelope,
    })
}

/// Derives the link id, meta and sort key of a to-be-added raw message.
///
/// Through [`io_pimdir::conventions`], the one implementation of pimdir SPEC
/// Annex A: two writers of one collection disagreeing about the id of a message
/// with no `Message-ID` link it twice and store its body twice, so the sync
/// engine and this client must derive it identically, not merely similarly.
fn derive(raw: &[u8]) -> Result<PimdirDerivation> {
    conventions::derive(MAIL_KIND, raw)
        .ok_or_else(|| anyhow!("pimdir has no conventions for `{MAIL_KIND}`"))
}

/// Streams `raw` into the blob store under `hash` and commits it durably,
/// returning the stored size.
fn write_blob(
    mut writer: io_pimdir::PimdirBlobWriter,
    raw: &[u8],
    hash: &io_replica::object::ReplicaHash,
) -> Result<u64> {
    use std::io::Write;

    writer.write_all(raw)?;
    Ok(writer.commit(hash)?)
}

/// Now, as the RFC 3339 stamp a queue row records.
fn stamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// Applies a flag op to a base set, producing the replacement set `SetFlags`
/// stores. An unknown base holds no markers to build on, so it reads as empty.
fn apply_flag_op(current: &ReplicaFlags, flags: &[Flag], op: FlagOp) -> ReplicaFlags {
    let incoming = flags.iter().map(|f| f.raw().to_string());
    match op {
        FlagOp::Set => ReplicaFlags::Known(incoming.collect()),
        FlagOp::Add => {
            let mut set = current.known().cloned().unwrap_or_default();
            set.extend(incoming);
            ReplicaFlags::Known(set)
        }
        FlagOp::Remove => {
            let mut set = current.known().cloned().unwrap_or_default();
            for flag in incoming {
                set.remove(&flag);
            }
            ReplicaFlags::Known(set)
        }
    }
}

/// A shared flag slice to replica flags (raw wire spellings).
fn to_replica_flags(flags: &[Flag]) -> ReplicaFlags {
    ReplicaFlags::Known(flags.iter().map(|f| f.raw().to_string()).collect())
}

/// Parses a message id — the public `seq` (a small integer) — from the CLI, with
/// a clear error for a non-numeric one.
fn parse_id(id: &str) -> Result<i64> {
    id.parse::<i64>()
        .map_err(|_| anyhow!("Invalid message id `{id}` (expected a number)"))
}

/// 1-indexed in-memory pagination; `page_size = None` returns the full slice.
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
    use std::collections::BTreeSet;

    use io_replica::placement::{ReplicaLinkId, ReplicaMeta};

    use super::*;

    #[test]
    fn envelope_is_built_from_meta_without_a_body() {
        let item = PimdirItem {
            seq: 42,
            link_id: ReplicaLinkId("x@y".into()),
            flags: ReplicaFlags::Known(["\\Seen".to_string()].into_iter().collect()),
            meta: Some(ReplicaMeta(
                r#"{"v":1,"message_id":"x@y","subject":"Hi","from":"a@x.org","to":"b@x.org","size":99}"#
                    .into(),
            )),
            sort_key: String::new(),
            object: None,
            level: io_replica::placement::ReplicaLevel::Meta,
            retention: None,
        };
        let envelope = envelope_from_item(&item);
        assert_eq!(envelope.id, "42");
        assert_eq!(envelope.subject, "Hi");
        assert_eq!(envelope.from[0].email, "a@x.org");
        assert_eq!(envelope.to[0].email, "b@x.org");
        assert_eq!(envelope.size, 99);
        assert!(envelope.flags.iter().any(|f| f.raw() == "\\Seen"));
    }

    /// A mailbox holding one `Message-ID` twice is ordinary (a double delivery,
    /// a retried append, a restore, a copy of a sent message), and the store
    /// keys the second copy apart under a minted `dup:<hint>#<handle>` (pimdir
    /// SPEC §9) rather than keeping one of the two. Both project as ordinary
    /// envelopes: the id a user sees is the `seq`, which differs between them,
    /// so neither hides the other, and the minted key never shows.
    #[test]
    fn two_items_sharing_a_message_id_project_two_public_ids() {
        let item = |seq, link_id: &str| PimdirItem {
            seq,
            link_id: ReplicaLinkId(link_id.into()),
            flags: ReplicaFlags::Known(BTreeSet::new()),
            meta: Some(ReplicaMeta(
                r#"{"v":1,"message_id":"twice@host","subject":"Twice","from":"a@x.org","size":10}"#
                    .into(),
            )),
            sort_key: String::new(),
            object: None,
            level: ReplicaLevel::Meta,
            retention: None,
        };

        let bare = envelope_from_item(&item(11, "twice@host"));
        let minted = envelope_from_item(&item(12, "dup:twice@host#1174"));

        assert_eq!(bare.message_id.as_deref(), Some("twice@host"));
        assert_eq!(bare.message_id, minted.message_id);
        assert_eq!(bare.id, "11");
        assert_eq!(minted.id, "12");
        assert!(!minted.id.contains("dup:"), "got {}", minted.id);
    }

    /// Markers nobody has read are not markers nobody holds: an item enumerated
    /// but never fetched must not render as a message with no flags at all.
    #[test]
    fn an_unread_flag_set_renders_as_no_flags_rather_than_panicking() {
        let item = PimdirItem {
            seq: 1,
            link_id: ReplicaLinkId("x@y".into()),
            flags: ReplicaFlags::Unknown,
            meta: None,
            sort_key: String::new(),
            object: None,
            level: io_replica::placement::ReplicaLevel::Probed,
            retention: None,
        };
        assert!(envelope_from_item(&item).flags.is_empty());
    }

    /// An added message links the way the store spells it: the bare
    /// `Message-ID` pimdir SPEC Annex A.1 gives, with nothing prepended, which
    /// is what the sync engine derives for the same body. Staging any other
    /// spelling would file a message the collection already holds as a second
    /// item instead of parking it (pimdir SPEC §15.3), the answer the store
    /// owes a producer that named a key it does not own; minting a second key
    /// is what it does for a source, not for this client.
    #[test]
    fn an_added_message_links_the_way_the_store_spells_it() {
        let raw = b"Message-ID: <new@host>\r\nSubject: Compose\r\nFrom: a@x.org\r\n\r\nbody";
        let derived = derive(raw).unwrap();
        assert_eq!(derived.link_id.0, "new@host");
        assert!(derived.meta.0.contains("\"v\":1"));
        assert!(derived.meta.0.contains("Compose"));
    }

    /// A message with no `Message-ID` falls back to the marked id, which is
    /// the one case a prefix names.
    #[test]
    fn a_message_without_a_message_id_keeps_the_alt_fallback() {
        let raw = b"Subject: No id\r\nFrom: a@x.org\r\n\r\nbody";
        let derived = derive(raw).unwrap();
        assert!(
            derived.link_id.0.starts_with("alt:"),
            "got {}",
            derived.link_id.0,
        );
    }

    #[test]
    fn flag_ops_add_set_and_remove() {
        let base = ReplicaFlags::Known(["\\Seen".to_string()].into_iter().collect());
        let flagged = [Flag::from_raw("\\Flagged")];
        let added = apply_flag_op(&base, &flagged, FlagOp::Add);
        assert!(added.contains("\\Seen") && added.contains("\\Flagged"));
        let set = apply_flag_op(&base, &flagged, FlagOp::Set);
        assert!(!set.contains("\\Seen") && set.contains("\\Flagged"));
        let seen = [Flag::from_raw("\\Seen")];
        let removed = apply_flag_op(&base, &seen, FlagOp::Remove);
        assert!(!removed.contains("\\Seen"));
    }

    /// An add onto an unknown set must not carry the unknown forward: the action
    /// replaces the set, and an unknown one would erase the markers a sync knows.
    #[test]
    fn a_flag_op_on_an_unknown_set_stages_a_known_one() {
        let staged = apply_flag_op(
            &ReplicaFlags::Unknown,
            &[Flag::from_raw("\\Seen")],
            FlagOp::Add,
        );
        assert_eq!(staged.known().map(|f| f.len()), Some(1));
        assert!(staged.contains("\\Seen"));
    }
    #[test]
    fn a_queued_creation_renders_as_mail_with_no_id() {
        let queued = PimdirPendingAction {
            id: 7,
            created_at: "2026-08-27T10:00:00Z".into(),
            producer: "himalaya".into(),
            action: PimdirAction::Add {
                link_id: Some(ReplicaLinkId("draft@x.org".into())),
                flags: ReplicaFlags::Known(["\\Draft".to_string()].into_iter().collect()),
                object: None,
                meta: Some(ReplicaMeta(
                    r#"{"v":1,"message_id":"draft@x.org","subject":"Re: lunch","to":"alice@x.org","size":12}"#
                        .into(),
                )),
                handle: None,
            },
            attempts: 0,
        };

        let queued = queued_from_action(&queued).unwrap();

        assert_eq!(queued.id, 7);
        assert_eq!(queued.created_at, "2026-08-27T10:00:00Z");
        assert_eq!(queued.envelope.subject, "Re: lunch");
        assert_eq!(queued.envelope.to[0].email, "alice@x.org");
        assert_eq!(queued.envelope.message_id.as_deref(), Some("draft@x.org"));
        // The row id names an action, not a message: putting it in `id` would
        // be an identifier from another space in the field commands read back.
        assert!(queued.envelope.id.is_empty());
    }

    #[test]
    fn only_a_queued_creation_renders_as_mail() {
        let queued = PimdirPendingAction {
            id: 8,
            created_at: "2026-08-27T10:00:00Z".into(),
            producer: "himalaya".into(),
            action: PimdirAction::Remove { seq: 42 },
            attempts: 0,
        };

        // A staged removal addresses a message that exists, so it shows in the
        // ordinary listing (as an absence) and has nothing to render here.
        assert!(queued_from_action(&queued).is_none());
    }
}
