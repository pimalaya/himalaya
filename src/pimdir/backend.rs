//! pimdir adapter for the shared cross-protocol client.
//!
//! Reads project [`io_pimdir`]'s client read API (`list_collections`,
//! `list_items`, `get_item`, `count_items`) plus the blob store, building
//! envelopes from the stored `v: 1` meta (pimdir SPEC §13) with no body reads.
//! An item whose body is not local (`level < Full`) still lists; `get_message`
//! reports "body not fetched" rather than an error — the cue to sync.
//!
//! Writes stage io-replica [`ReplicaMutation`]s through the store's `mutate`
//! seam (never raw SQL), so the next sync derives and pushes them. A write is
//! attributed to the client's configured source; it fails loudly when the store
//! was not synced as that source (no binding for the item), rather than silently
//! staging a change no sync will carry.

use anyhow::{Result, anyhow, bail};
use chrono::DateTime;
use io_pimdir::PimdirItem;
use io_replica::{
    client::ReplicaStorage,
    collection::ReplicaCollectionId,
    coroutine::{ReplicaArg, ReplicaCoroutine, ReplicaCoroutineState, ReplicaYield},
    mutate::{ReplicaMutate, ReplicaMutation},
    object::ReplicaObject,
    placement::{ReplicaFlags, ReplicaHandle, ReplicaLinkId, ReplicaMeta, ReplicaPlacement},
};
use log::warn;
use mail_parser::{Address as MailParserAddress, MessageParser};
use serde::Deserialize;

use crate::{
    email::{
        address::Address,
        envelope::Envelope,
        flag::{Flag, FlagOp, IanaFlag},
        mailbox::Mailbox,
        search::{eval, query::SearchEmailsQuery},
    },
    pimdir::{client::PimdirClient, hash::content_hash},
};

/// The mail media type a pimdir collection carries to be a mailbox.
const MAIL_KIND: &str = "message/rfc822";

/// How many items to pull per keyset page when scanning a whole collection.
const SCAN_BATCH: usize = 500;

/// A reader's view of the `message/rfc822` meta (pimdir SPEC §13).
#[derive(Default, Deserialize)]
struct MetaView {
    #[serde(default)]
    message_id: Option<String>,
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
    // --- Reads (source-independent: they observe the shared items) -------

    /// Lists the mail collections (declared `message/rfc822`, or kind-less
    /// legacy ones), sorted by name. `with_counts` fills `total` with the live
    /// item count.
    pub fn list_mailboxes(&mut self, with_counts: bool) -> Result<Vec<Mailbox>> {
        let mut mailboxes = Vec::new();
        for collection in self.store.list_collections()? {
            if !collection.kind.is_empty() && collection.kind != MAIL_KIND {
                continue;
            }
            let total = if with_counts {
                Some(self.store.count_items(&collection.id)?)
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
        mailboxes.sort_by(|a, b| a.name.cmp(&b.name));
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
        let mut envelopes: Vec<Envelope> = self
            .scan_items(mailbox)?
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
        let filter = query.and_then(|q| q.filter.as_ref());
        let mut hits: Vec<Envelope> = self
            .scan_items(mailbox)?
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

    /// Reads one message's raw bytes from its content-addressed blob. Fails with
    /// a clear "body not fetched" when the item is not hydrated to `Full` (no
    /// local body) — the client's cue to sync rather than a data-loss error.
    pub fn get_message(&mut self, mailbox: &str, id: &str, seen: bool) -> Result<Vec<u8>> {
        let Some(item) = self.store.get_item(mailbox, parse_id(id)?)? else {
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
        // and a store not synced as this source must not fail the read.
        if seen {
            let seen_flag = Flag::from_iana(IanaFlag::Seen);
            if let Err(err) = self.store_flags(mailbox, &[id], &[seen_flag], FlagOp::Add) {
                warn!("could not stage \\Seen on `{id}` in `{mailbox}`: {err:#}");
            }
        }

        Ok(bytes)
    }

    // --- Writes (staged mutations a later sync pushes) -------------------

    /// Adds, sets, or removes `flags` on an id set, staged as `SetFlags`.
    pub fn store_flags(
        &mut self,
        mailbox: &str,
        ids: &[&str],
        flags: &[Flag],
        op: FlagOp,
    ) -> Result<()> {
        for id in ids {
            let placement = self.synced_placement(mailbox, id)?;
            let next = apply_flag_op(&placement.flags, flags, op);
            self.run_mutation(
                mailbox,
                ReplicaMutation::SetFlags {
                    handle: placement.handle,
                    flags: next,
                },
            )?;
        }
        Ok(())
    }

    /// Appends `raw` to `mailbox` as a locally-authored item, staged as `Add`
    /// (the next sync uploads it). Returns the link id it is stored under.
    pub fn add_message(&mut self, mailbox: &str, flags: &[Flag], raw: Vec<u8>) -> Result<String> {
        let (link_id, meta) = derive_link_and_meta(&raw);
        let object = ReplicaObject {
            hash: content_hash(&raw),
            size: raw.len(),
        };
        let handle = ReplicaHandle(format!("local:{}", link_id.0));
        let link = link_id.0.clone();
        self.run_mutation(
            mailbox,
            ReplicaMutation::Add {
                handle,
                link_id,
                flags: to_replica_flags(flags),
                object,
                body: raw,
                meta: Some(meta),
            },
        )?;
        // The store assigned the item its public id on insert; return that.
        let seq = self
            .store
            .seq_for_link(mailbox, &link)?
            .ok_or_else(|| anyhow!("Added message `{link}` in `{mailbox}` has no public id"))?;
        Ok(seq.to_string())
    }

    /// Copies each id from `from` to `to`, staged as `Copy` (a server-side copy
    /// on the next sync, no body re-upload).
    pub fn copy_messages(&mut self, from: &str, to: &str, ids: &[&str]) -> Result<usize> {
        for id in ids {
            let placement = self.synced_placement(from, id)?;
            let placeholder = ReplicaHandle(format!("copy:{}:{}", to, placement.handle.0));
            self.run_mutation(
                from,
                ReplicaMutation::Copy {
                    handle: placement.handle,
                    target: ReplicaCollectionId(to.to_string()),
                    placeholder,
                },
            )?;
        }
        Ok(ids.len())
    }

    /// Moves each id from `from` to `to`, staged as `Move` (one server-side move
    /// on the next sync).
    pub fn move_messages(&mut self, from: &str, to: &str, ids: &[&str]) -> Result<usize> {
        for id in ids {
            let placement = self.synced_placement(from, id)?;
            let placeholder = ReplicaHandle(format!("move:{}:{}", to, placement.handle.0));
            self.run_mutation(
                from,
                ReplicaMutation::Move {
                    handle: placement.handle,
                    target: ReplicaCollectionId(to.to_string()),
                    placeholder,
                },
            )?;
        }
        Ok(ids.len())
    }

    /// Deletes each id from `mailbox`, staged as `Remove` (a tombstone the next
    /// sync pushes as an expunge).
    pub fn delete_messages(&mut self, mailbox: &str, ids: &[&str]) -> Result<()> {
        for id in ids {
            let placement = self.synced_placement(mailbox, id)?;
            self.run_mutation(mailbox, ReplicaMutation::Remove(placement.handle))?;
        }
        Ok(())
    }

    // --- Internals ------------------------------------------------------

    /// Pulls every live item of a collection by keyset paging (the read API is
    /// paginated; the shared list/search commands sort and paginate in memory,
    /// as the file backends do).
    fn scan_items(&self, mailbox: &str) -> Result<Vec<PimdirItem>> {
        let mut all = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let page = self
                .store
                .list_items(mailbox, cursor.as_deref(), SCAN_BATCH)?;
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

    /// The source's placement for the public id `id`, guaranteed to carry a sync
    /// base.
    ///
    /// Resolves the public `seq` to the internal `link_id` first, then finds the
    /// placement by link id. A change on a placement with no base would stage as a
    /// fresh create rather than an edit and no sync would carry it, so this is the
    /// guard that turns a misconfigured source (the store was never synced as
    /// `self.source`) into a clear error instead of a silent no-op.
    fn synced_placement(&self, collection: &str, id: &str) -> Result<ReplicaPlacement> {
        let seq = parse_id(id)?;
        let link_id = self
            .store
            .get_item(collection, seq)?
            .map(|item| item.link_id.0)
            .ok_or_else(|| anyhow!("Message `{id}` not found in `{collection}`"))?;
        let loaded = self
            .store
            .load(&ReplicaCollectionId(collection.to_string()))?;
        let placement = loaded
            .placements
            .into_iter()
            .find(|p| p.link_id.as_ref().map(|l| l.0.as_str()) == Some(link_id.as_str()))
            .ok_or_else(|| anyhow!("Message `{id}` not found in `{collection}`"))?;
        if placement.base.is_none() {
            bail!(
                "`{collection}` was not synced as source `{}`, so `{id}` cannot be edited \
                 here; set `pimdir.source` to the sync source and sync first",
                self.source
            );
        }
        Ok(placement)
    }

    /// Drives a `mutate` coroutine to completion against the store: it only ever
    /// asks to load the collection and to write the staged ops.
    fn run_mutation(&mut self, collection: &str, mutation: ReplicaMutation) -> Result<()> {
        let mut coroutine = ReplicaMutate::new(collection.to_string(), mutation);
        let mut arg: Option<ReplicaArg> = None;
        loop {
            match coroutine.resume(arg.take()) {
                ReplicaCoroutineState::Yielded(ReplicaYield::WantsLoad(collection)) => {
                    let loaded = self.store.load(&collection)?;
                    arg = Some(ReplicaArg::Load(loaded));
                }
                ReplicaCoroutineState::Yielded(ReplicaYield::WantsWrite(ops)) => {
                    self.store.write(ops)?;
                    arg = Some(ReplicaArg::Write);
                }
                ReplicaCoroutineState::Yielded(_) => {
                    bail!("pimdir mutate asked for an unexpected step");
                }
                ReplicaCoroutineState::Complete(result) => {
                    return result.map_err(|err| anyhow!("pimdir mutate failed: {err}"));
                }
            }
        }
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
        .0
        .iter()
        .map(|raw| Flag::from_raw(raw.clone()))
        .collect();
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
        // The public id: a short per-collection integer, not the long link id.
        id: item.seq.to_string(),
        message_id: view.message_id,
        flags,
        subject: view.subject,
        from,
        to,
        date,
        size: view.size.unwrap_or(0),
        has_attachment: None,
    }
}

/// Derives the link id and `v: 1` meta of a to-be-added raw message, matching
/// Neverest's writer so an added message deduplicates against a synced one.
fn derive_link_and_meta(raw: &[u8]) -> (ReplicaLinkId, ReplicaMeta) {
    let parsed = MessageParser::default().parse(raw);
    let message_id = parsed.as_ref().and_then(|m| m.message_id());
    let subject = parsed.as_ref().and_then(|m| m.subject()).unwrap_or("");
    let from = parsed
        .as_ref()
        .and_then(|m| m.from())
        .and_then(first_address);
    let to = parsed.as_ref().and_then(|m| m.to()).and_then(first_address);
    let date = parsed
        .as_ref()
        .and_then(|m| m.date())
        .map(|d| d.to_rfc3339());

    let link_id = match message_id {
        Some(mid) if !mid.trim().is_empty() => {
            ReplicaLinkId(format!("mid:{}", mid.trim().trim_matches(['<', '>'])))
        }
        _ => ReplicaLinkId(format!(
            "alt:{subject}|{}|{}",
            date.as_deref().unwrap_or(""),
            from.as_deref().unwrap_or("")
        )),
    };

    #[derive(serde::Serialize)]
    struct MetaWrite<'a> {
        v: u8,
        #[serde(skip_serializing_if = "Option::is_none")]
        message_id: Option<&'a str>,
        subject: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        from: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        to: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        date: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        size: Option<u64>,
    }
    let summary = MetaWrite {
        v: 1,
        message_id,
        subject,
        from: from.as_deref(),
        to: to.as_deref(),
        date: date.as_deref(),
        size: (!raw.is_empty()).then_some(raw.len() as u64),
    };
    let meta = ReplicaMeta(serde_json::to_string(&summary).unwrap_or_default());
    (link_id, meta)
}

/// The first address of a mail-parser header group, as a plain string.
fn first_address(addrs: &MailParserAddress<'_>) -> Option<String> {
    addrs
        .clone()
        .into_list()
        .into_iter()
        .find_map(|a| a.address.map(|s| s.into_owned()))
}

/// Applies a flag op to a base set, producing the replacement set `SetFlags`
/// stores.
fn apply_flag_op(current: &ReplicaFlags, flags: &[Flag], op: FlagOp) -> ReplicaFlags {
    let incoming = flags.iter().map(|f| f.raw().to_string());
    match op {
        FlagOp::Set => ReplicaFlags(incoming.collect()),
        FlagOp::Add => {
            let mut set = current.0.clone();
            set.extend(incoming);
            ReplicaFlags(set)
        }
        FlagOp::Remove => {
            let mut set = current.0.clone();
            for flag in incoming {
                set.remove(&flag);
            }
            ReplicaFlags(set)
        }
    }
}

/// A shared flag slice to replica flags (raw wire spellings).
fn to_replica_flags(flags: &[Flag]) -> ReplicaFlags {
    ReplicaFlags(flags.iter().map(|f| f.raw().to_string()).collect())
}

/// Parses a message id — the public per-collection `seq` (a small integer) — from
/// the CLI, with a clear error for a non-numeric one.
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
    use super::*;

    #[test]
    fn envelope_is_built_from_meta_without_a_body() {
        let item = PimdirItem {
            seq: 42,
            link_id: ReplicaLinkId("mid:x@y".into()),
            flags: ReplicaFlags(["\\Seen".to_string()].into_iter().collect()),
            meta: Some(ReplicaMeta(
                r#"{"v":1,"message_id":"x@y","subject":"Hi","from":"a@x.org","to":"b@x.org","size":99}"#
                    .into(),
            )),
            object: None,
            level: io_replica::placement::ReplicaLevel::Meta,
        };
        let envelope = envelope_from_item(&item);
        assert_eq!(envelope.id, "42");
        assert_eq!(envelope.subject, "Hi");
        assert_eq!(envelope.from[0].email, "a@x.org");
        assert_eq!(envelope.to[0].email, "b@x.org");
        assert_eq!(envelope.size, 99);
        assert!(envelope.flags.iter().any(|f| f.raw() == "\\Seen"));
    }

    #[test]
    fn add_derives_a_mid_link_and_v1_meta() {
        let raw = b"Message-ID: <new@host>\r\nSubject: Compose\r\nFrom: a@x.org\r\n\r\nbody";
        let (link, meta) = derive_link_and_meta(raw);
        assert_eq!(link.0, "mid:new@host");
        assert!(meta.0.contains("\"v\":1"));
        assert!(meta.0.contains("Compose"));
    }

    #[test]
    fn flag_ops_add_set_and_remove() {
        let base = ReplicaFlags(["\\Seen".to_string()].into_iter().collect());
        let flagged = [Flag::from_raw("\\Flagged")];
        let added = apply_flag_op(&base, &flagged, FlagOp::Add);
        assert!(added.contains("\\Seen") && added.contains("\\Flagged"));
        let set = apply_flag_op(&base, &flagged, FlagOp::Set);
        assert!(!set.contains("\\Seen") && set.contains("\\Flagged"));
        let seen = [Flag::from_raw("\\Seen")];
        let removed = apply_flag_op(&base, &seen, FlagOp::Remove);
        assert!(!removed.contains("\\Seen"));
    }
}
