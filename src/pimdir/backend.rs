//! # pimdir backend
//!
//! The pimdir adapter of the shared cross-protocol client, reading the
//! store's index and blobs.
//!
//! Envelopes are built from the stored mail summary with no body read,
//! so an item whose body is not local still lists and reads as not
//! fetched rather than as an error.
//!
//! Writes enqueue actions for the store's owner to apply. Himalaya is a
//! producer, never the owner: it mutates no index and collects nothing,
//! so a staged flag change cannot race a sync mid-hydration.
//!
//! A mailbox is its collection id, verbatim. The sync binds a source's
//! collections under a namespace, so the mailbox a server calls `INBOX`
//! is `imap/INBOX` here. The id is opaque to the store, so shortening it
//! would guess at the sync's convention rather than look it up, and
//! `mailbox.alias` is how a user avoids typing it.

use std::{io::Write, ops::ControlFlow};

use anyhow::{Result, anyhow, bail};
use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime};
use io_pimdir::{
    capability,
    client::{
        blobs::PimdirBlobWriter,
        producer::{PimdirActionStatus, PimdirPendingAction},
        reader::{
            PimdirCollection, PimdirItem, PimdirMailCursor, PimdirMailFilter, PimdirRoundState,
        },
    },
    codec::PimdirAction,
    collection::{PimdirCollectionId, PimdirCoverage},
    object::{PimdirHash, PimdirObject},
    placement::PimdirFlags,
    summary::{
        PimdirAddress, PimdirSummary,
        mail::{self, PimdirMailSummary},
    },
};
use log::{info, warn};
use serde::Serialize;

use crate::{
    email::{
        address::Address,
        envelope::Envelope,
        flag::{Flag, FlagOp, IanaFlag},
        mailbox::{Mailbox, MailboxRole},
        search::{
            eval,
            filter::query::SearchEmailsFilterQuery,
            query::SearchEmailsQuery,
            sort::query::{SearchEmailsSorter, SearchEmailsSorterKind, SearchEmailsSorterOrder},
        },
        submission::SubmissionEnvelope,
    },
    error::{CodedError, ErrorCode},
    pimdir::client::PimdirClient,
};

/// The mail media type a pimdir collection carries to be a mailbox.
const MAIL_KIND: &str = "message/rfc822";

/// The queue action kind of a message for the store's owner to send.
const SUBMIT: &str = "submit";

/// How many items to pull per keyset page when scanning a whole collection.
const SCAN_BATCH: usize = 500;

/// The smallest keyset page a filtered search reads, so a sparse match
/// does not cost a query per row.
const WALK_MIN: usize = 64;

/// The sort key of a dated mail: its summary date (pimdir STORAGE Annex A.1).
const KEY_FORMAT: &str = "%Y-%m-%dT%H:%M:%SZ";

/// Whether a collection's declared kind makes it a mailbox, matched on
/// the bare media type the way the store reads it.
pub(crate) fn is_mail(kind: &str) -> bool {
    kind.split(';').next().unwrap_or_default().trim() == MAIL_KIND
}

impl PimdirClient {
    /// The collection a user-typed mailbox addresses, which is the mailbox
    /// itself: a pimdir mailbox is its collection id, verbatim.
    ///
    /// The id is checked against the account's mail collections rather than
    /// passed through, an id nothing was written under otherwise reading as
    /// a mailbox that exists and is empty.
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
                role: collection.role.as_deref().map(MailboxRole::parse),
                total,
                unread: None,
            });
        }
        mailboxes.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(mailboxes)
    }

    /// Lists the account's mailboxes as [`Self::list_mailboxes`] does with
    /// counts, each with the coverage the store lists it with and the round
    /// a source has under way.
    ///
    /// The coverage is the narrowest of the collection's sources' (pimdir
    /// STORAGE §14.1), `None` while one never closed a round, so a reader
    /// says "mail since" from it and knows a search below it is not
    /// exhaustive. A store its owner has not reconciled yet has none.
    pub fn list_covered_mailboxes(&mut self) -> Result<Vec<PimdirCoveredMailbox>> {
        let collections = self.mail_collections()?;
        let ids: Vec<&str> = collections.iter().map(|c| c.id.as_str()).collect();
        let unread = self
            .store
            .count_unread(&ids, None)
            .map_err(|err| anyhow!("Count unread pimdir mail: {err}"))?;

        let mut mailboxes = Vec::new();
        for collection in collections {
            let total = self
                .store
                .count_items(&collection.id)
                .map_err(|err| anyhow!("Count items in `{}`: {err}", collection.id))?;
            let round = self
                .store
                .list_coverage(&collection.id)
                .map_err(|err| anyhow!("Read the coverage of `{}`: {err}", collection.id))?
                .into_iter()
                .find_map(|source| source.round);
            mailboxes.push(PimdirCoveredMailbox {
                mailbox: Mailbox {
                    unread: Some(unread.get(&collection.id).copied().unwrap_or(0)),
                    id: collection.id,
                    name: collection.name,
                    role: collection.role.as_deref().map(MailboxRole::parse),
                    total: Some(total),
                },
                coverage: collection.coverage,
                round,
            });
        }
        mailboxes.sort_by(|a, b| a.mailbox.id.cmp(&b.mailbox.id));
        Ok(mailboxes)
    }

    /// Lists envelopes from `mailbox`, built from the stored summaries (no
    /// body reads), in the store's newest-first order then paginated.
    ///
    /// With a page size, the store is read by keyset pages up to the end of
    /// the page asked for, never past it: page N of size S reads N × S items.
    pub fn list_envelopes(
        &mut self,
        mailbox: &str,
        page: Option<u32>,
        page_size: Option<u32>,
        _with_attachment: bool,
    ) -> Result<Vec<Envelope>> {
        let collection = self.hub_id(mailbox)?;
        let Some(wanted) = page_end(page, page_size) else {
            let envelopes = self
                .scan_items(&collection)?
                .iter()
                .map(envelope_from_item)
                .collect();
            return Ok(envelopes);
        };
        if wanted == 0 {
            return Ok(Vec::new());
        }

        let mut envelopes: Vec<Envelope> = Vec::new();
        self.walk(&collection, None, false, wanted.min(SCAN_BATCH), |item| {
            envelopes.push(envelope_from_item(&item));
            match wanted - envelopes.len() {
                0 => ControlFlow::Break(()),
                left => ControlFlow::Continue(left.min(SCAN_BATCH)),
            }
        })?;
        Ok(paginate(envelopes, page, page_size))
    }

    /// Searches envelopes in `mailbox`: builds them from the summaries,
    /// applies the shared filter/sort/paginate. Body clauses cannot match on
    /// an item whose body is not local (no bytes to scan); header/flag
    /// clauses always do.
    ///
    /// The store is read newest first, within the date bounds the query
    /// states and through the unread chip where it applies, and the read
    /// stops at the end of the page when the order is the store's own
    /// ([`PimdirPushdown`]). Every clause is still matched in memory.
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
        let sort = query.and_then(|q| q.sort.as_deref());

        match self.search_pushed(&collection, filter, sort, page, page_size)? {
            Some(hits) => Ok(hits),
            None => self.search_scanned(&collection, filter, sort, page, page_size),
        }
    }

    /// [`Self::search_envelopes`] reading only the rows the query can
    /// reach, `None` when a row read breaks the store's ordering contract
    /// (its sort key is not its summary date), which the bounds and the
    /// early stop rest on.
    fn search_pushed(
        &self,
        collection: &str,
        filter: Option<&SearchEmailsFilterQuery>,
        sort: Option<&[SearchEmailsSorter]>,
        page: Option<u32>,
        page_size: Option<u32>,
    ) -> Result<Option<Vec<Envelope>>> {
        if page_size == Some(0) {
            return Ok(Some(Vec::new()));
        }

        let pushdown = filter.map(PimdirPushdown::of).unwrap_or_default();
        let unread = pushdown.unread && self.overlay_quiet(collection);
        // NOTE: a page of a search in another order sits anywhere in the
        // range, so the whole range is read then sorted.
        let wanted = match is_store_order(sort) {
            true => page_end(page, page_size),
            false => None,
        };
        let batch =
            |left: Option<usize>| left.map_or(SCAN_BATCH, |left| left.clamp(WALK_MIN, SCAN_BATCH));

        let start = pushdown.before.as_deref().map(|key| (key, 0));
        let mut hits: Vec<Envelope> = Vec::new();
        let mut ordered = true;
        self.walk(collection, start, unread, batch(wanted), |item| {
            if !in_store_order(&item) {
                ordered = false;
                return ControlFlow::Break(());
            }
            if let Some(from) = &pushdown.from
                && item.sort_key.as_str() < from.as_str()
            {
                return ControlFlow::Break(());
            }
            let envelope = envelope_from_item(&item);
            if filter.is_none_or(|filter| eval::matches_filter(&envelope, &[], filter)) {
                hits.push(envelope);
            }
            let left = wanted.map(|wanted| wanted.saturating_sub(hits.len()));
            match left {
                Some(0) => ControlFlow::Break(()),
                left => ControlFlow::Continue(batch(left)),
            }
        })?;

        if !ordered {
            warn!("a row of `{collection}` is not filed under its date, searching it whole");
            return Ok(None);
        }
        eval::sort_envelopes(&mut hits, sort);
        Ok(Some(paginate(hits, page, page_size)))
    }

    /// [`Self::search_envelopes`] over every item of the mailbox, matched,
    /// sorted then paginated in memory.
    fn search_scanned(
        &self,
        collection: &str,
        filter: Option<&SearchEmailsFilterQuery>,
        sort: Option<&[SearchEmailsSorter]>,
        page: Option<u32>,
        page_size: Option<u32>,
    ) -> Result<Vec<Envelope>> {
        let mut hits: Vec<Envelope> = self
            .scan_items(collection)?
            .iter()
            .map(envelope_from_item)
            .filter(|envelope| match filter {
                Some(filter) => eval::matches_filter(envelope, &[], filter),
                None => true,
            })
            .collect();
        eval::sort_envelopes(&mut hits, sort);
        Ok(paginate(hits, page, page_size))
    }

    /// The mailbox's queued creations and sends, rendered as mail.
    ///
    /// The operator CLI is kind-agnostic and prints ids, hashes and flags.
    /// A queued action carries no summary, the owner deriving one from the
    /// body when it applies the action, so this reads the body the action
    /// pins and derives the same way.
    pub fn queued_envelopes(&mut self, mailbox: &str) -> Result<Vec<PimdirQueued>> {
        let queued = self.queued_mail(mailbox)?;

        let mut rows = Vec::new();
        for action in &queued {
            let body = match action.action.object_hash() {
                Some(hash) => self.blobs.get(hash)?,
                None => None,
            };
            rows.extend(queued_from_action(action, body.as_deref()));
        }
        Ok(rows)
    }

    /// Reads one message's raw bytes from its content-addressed blob.
    ///
    /// An item with no local body fails as not fetched rather than as a data
    /// loss, which is the cue to sync.
    pub fn get_message(&mut self, mailbox: &str, id: &str, seen: bool) -> Result<Vec<u8>> {
        let collection = self.hub_id(mailbox)?;
        let Some(item) = self.get(&collection, id)? else {
            bail!("Message `{id}` not found in `{mailbox}`");
        };
        let Some(hash) = item.object else {
            return Err(CodedError::new(
                ErrorCode::BodyPending,
                format!(
                    "Message `{id}` in `{mailbox}` is not downloaded yet (body not fetched); \
                     run a sync to hydrate it"
                ),
            )
            .into());
        };
        let bytes = self
            .blobs
            .get(&hash)?
            .ok_or_else(|| anyhow!("Body blob missing for `{id}` in `{mailbox}`"))?;

        // NOTE: a store refusing the staged flag change must not fail the
        // read, which is non-mutating by default.
        if seen {
            let seen_flag = Flag::from_iana(IanaFlag::Seen);
            if let Err(err) = self.store_flags(mailbox, &[id], &[seen_flag], FlagOp::Add) {
                warn!("could not stage \\Seen on `{id}` in `{mailbox}`: {err:#}");
            }
        }

        Ok(bytes)
    }

    /// Adds, sets, or removes `flags` on an id set, staged as `SetFlags`.
    ///
    /// The action carries the whole replacement set, never a delta, so an
    /// owner applying it twice lands the same state.
    pub fn store_flags(
        &mut self,
        mailbox: &str,
        ids: &[&str],
        flags: &[Flag],
        op: FlagOp,
    ) -> Result<()> {
        let collection = self.hub_id(mailbox)?;
        let mut producer = self.producer()?;

        for id in ids {
            let seq = self.seq(&collection, id)?;
            let current = self
                .get(&collection, id)?
                .map(|item| item.flags)
                .unwrap_or(PimdirFlags::Unknown);
            let action = PimdirAction::SetFlags {
                seq,
                flags: apply_flag_op(&current, flags, op),
            };
            self.enqueue(&mut producer, &collection, &action, None)
                .map_err(|err| anyhow!("Stage flags on `{id}` in `{mailbox}`: {err}"))?;
        }
        Ok(())
    }

    /// Stages a locally-authored message for the next sync to upload,
    /// returning its queue row and the link id it is stored under.
    ///
    /// The body lands in the blob store durably before the action referencing
    /// it is enqueued, and the queue row pins the object, so nothing collects
    /// a body between the two.
    ///
    /// The link id is the bare `Message-ID`, derived the way the owner
    /// derives it from the same body, and a staged add whose link id the
    /// collection already holds parks rather than deduplicate or mint a
    /// key of its own.
    ///
    /// The store answers its two producers differently on purpose. It mints
    /// for a source, a replica owing the collection what the collection
    /// holds, so one `Message-ID` twice keeps two items. It parks for a
    /// producer, which named a key it does not own and is told so rather than
    /// filed under one it never asked for.
    pub fn add_message(
        &mut self,
        mailbox: &str,
        flags: &[Flag],
        raw: Vec<u8>,
    ) -> Result<PimdirStaged> {
        let collection = self.hub_id(mailbox)?;
        let link_id = mail::derive(&raw).link_id;

        let hash = self.blobs.hash(&raw);
        let writer = self.blobs.writer()?;
        let size = write_blob(writer, &raw, &hash)?;
        let object = PimdirObject {
            hash,
            size: size as usize,
        };

        let mut producer = self.producer()?;
        let action = PimdirAction::Add {
            link_id: Some(link_id.clone()),
            flags: flags.iter().map(Flag::raw).collect(),
            object: Some(object.hash.clone()),
        };
        let queue_id = self
            .enqueue(&mut producer, &collection, &action, Some(&object))
            .map_err(|err| anyhow!("Stage add in `{mailbox}`: {err}"))?;

        Ok(PimdirStaged {
            queue_id,
            message_id: link_id.0,
        })
    }

    /// Queues a message for the store's owner to send.
    ///
    /// The body is stored as given, `Bcc:` included, and the row carries the
    /// envelope derived from its headers, the `submit` intent any owner of
    /// the store may perform. The owner holds the credentials and sends on its next run;
    /// nothing leaves from here.
    ///
    /// The row is filed under `mailbox`, which must exist: an enqueue
    /// creates the collection it names, and a send anchored on a made-up
    /// name would add a mailbox to the store.
    ///
    /// With `copy`, the intent asks the owner to file a copy in `mailbox`
    /// once the message is sent (pimdir STORAGE Annex B.2), so a failed
    /// send leaves no copy behind. An owner that declares nothing predates
    /// the field and would ignore it, so the copy is left to the caller
    /// there, as before.
    ///
    /// Returns the queue row, the message's link id, and whether the intent
    /// carries the copy.
    pub fn send_message(
        &mut self,
        mailbox: Option<&str>,
        raw: Vec<u8>,
        copy: bool,
    ) -> Result<PimdirSubmitted> {
        let Some(mailbox) = mailbox else {
            bail!(
                "A pimdir account queues a sent message under a mailbox: \
                 set `mailbox.alias.sent`, or pass one with `--save`"
            );
        };
        let collection = self.hub_id(mailbox)?;
        let envelope = SubmissionEnvelope::parse(&raw)?;

        let hash = self.blobs.hash(&raw);
        let writer = self.blobs.writer()?;
        let size = write_blob(writer, &raw, &hash)?;
        let object = PimdirObject {
            hash,
            size: size as usize,
        };

        let mut producer = self.producer()?;
        let source = self.performer(&producer, &collection, capability::MAIL_SUBMIT)?;
        let carried = copy && source.is_some();

        let payload = SubmitPayload {
            v: 1,
            object: &object.hash.0,
            from: &envelope.from,
            rcpts: &envelope.rcpts,
            subject: envelope.subject.as_deref(),
            source: source.as_deref(),
            copy: carried.then_some(collection.as_str()),
        };
        let action = PimdirAction::Unknown {
            kind: SUBMIT.to_owned(),
            payload: serde_json::to_string(&payload)?,
            object_hash: Some(object.hash.clone()),
        };

        let id = self
            .enqueue(&mut producer, &collection, &action, Some(&object))
            .map_err(|err| anyhow!("Queue send in `{mailbox}`: {err}"))?;
        info!("message queued for sending as action {id}, see `himalaya pimdir queue list`");
        Ok(PimdirSubmitted {
            staged: PimdirStaged {
                queue_id: id,
                message_id: mail::derive(&raw).link_id.0,
            },
            carried,
        })
    }

    /// Where queue row `id` stands (pimdir STORAGE §15.4): pending, parked,
    /// applied with the item an `add` created, or unknown. A row anchored on
    /// another account's collection reads as unknown.
    pub fn queue_row(&self, id: i64) -> Result<PimdirActionStatus> {
        let status = self
            .store
            .action_status(id)
            .map_err(|err| anyhow!("Read queue row {id}: {err}"))?;
        let collection = match &status {
            PimdirActionStatus::Pending { collection, .. }
            | PimdirActionStatus::Parked { collection, .. }
            | PimdirActionStatus::Applied { collection, .. } => collection,
            PimdirActionStatus::Unknown => return Ok(status),
        };

        let ours = self
            .store
            .list_collections_by_account(self.account.as_deref())
            .map_err(|err| anyhow!("List the account's collections: {err}"))?
            .iter()
            .any(|ours| &ours.id == collection);
        Ok(if ours {
            status
        } else {
            PimdirActionStatus::Unknown
        })
    }

    /// Copies each id from `from` to `to`, staged as `Copy` (a server-side copy
    /// on the next sync, no body re-upload).
    pub fn copy_messages(&mut self, from: &str, to: &str, ids: &[&str]) -> Result<usize> {
        self.refile(from, to, ids, |seq, target| PimdirAction::Copy {
            seq,
            to: target,
        })
    }

    /// Moves each id between two mailboxes, staged as one server-side move
    /// for the next sync.
    pub fn move_messages(&mut self, from: &str, to: &str, ids: &[&str]) -> Result<usize> {
        self.refile(from, to, ids, |seq, target| PimdirAction::Move {
            seq,
            to: target,
        })
    }

    /// Queues the creation of mailbox `name` for the store's owner, under
    /// the mailbox `parent` when given, returning the queue row and the
    /// source that performs it.
    ///
    /// The intent is anchored on `parent`, else on the account's first
    /// mailbox: a mailbox that does not exist yet has no collection to file
    /// a row under, and an enqueue on a made-up id would create one in the
    /// store. The new mailbox arrives with the sync that performs it.
    ///
    /// The performer is named as for a send. A store whose sources declare
    /// nothing has an owner predating the intent, which would never perform
    /// it, so the creation is refused there rather than left to wait.
    pub fn create_mailbox(&mut self, name: &str, parent: Option<&str>) -> Result<PimdirCreated> {
        // NOTE: a name the server already holds is its to refuse: the row
        // parks, and `pimdir queue show` says why.
        let parent = parent.map(|parent| self.hub_id(parent)).transpose()?;
        let anchor = match &parent {
            Some(parent) => parent.clone(),
            None => {
                let mut ids: Vec<String> =
                    self.mail_collections()?.into_iter().map(|c| c.id).collect();
                ids.sort();
                ids.into_iter()
                    .next()
                    .ok_or_else(|| anyhow!("The account holds no mailbox yet: run a sync first"))?
            }
        };

        let mut producer = self.producer()?;
        let Some(source) = self.performer(&producer, &anchor, capability::COLLECTION_CREATE)?
        else {
            bail!(
                "The sources of this store declare no capabilities, so its sync engine \
                 cannot create a mailbox"
            );
        };

        let queue_id = producer
            .enqueue_collection_create(&anchor, name, parent.as_deref(), Some(&source))
            .map_err(|err| anyhow!("Queue the creation of `{name}`: {err}"))?;
        Ok(PimdirCreated { queue_id, source })
    }

    /// Deletes each id from `mailbox`, staged as `Remove` (the next sync pushes
    /// it as the backend's own disposal).
    pub fn delete_messages(&mut self, mailbox: &str, ids: &[&str]) -> Result<()> {
        let collection = self.hub_id(mailbox)?;
        let mut producer = self.producer()?;

        for id in ids {
            let seq = self.seq(&collection, id)?;
            self.enqueue(
                &mut producer,
                &collection,
                &PimdirAction::Remove { seq },
                None,
            )
            .map_err(|err| anyhow!("Stage delete of `{id}` in `{mailbox}`: {err}"))?;
        }
        Ok(())
    }

    /// Stages one refiling action per id, `build` deciding whether the source
    /// copy stays.
    fn refile(
        &mut self,
        from: &str,
        to: &str,
        ids: &[&str],
        build: fn(i64, PimdirCollectionId) -> PimdirAction,
    ) -> Result<usize> {
        let source = self.hub_id(from)?;
        let target = PimdirCollectionId(self.hub_id(to)?);
        let mut producer = self.producer()?;

        for id in ids {
            let seq = self.seq(&source, id)?;
            self.enqueue(&mut producer, &source, &build(seq, target.clone()), None)
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

    /// Walks a mailbox newest first by keyset pages, from below `start`
    /// (a sort key and a public id) or from the newest, handing each item
    /// to `visit`, which stops the walk or names the size of the next page;
    /// the walk ends with the mailbox.
    ///
    /// `unread` reads through the store's unread chip, which reads the
    /// committed rows only: the caller checks no queued action changes what
    /// the mailbox shows ([`Self::overlay_quiet`]).
    fn walk(
        &self,
        collection: &str,
        start: Option<(&str, i64)>,
        unread: bool,
        first: usize,
        mut visit: impl FnMut(PimdirItem) -> ControlFlow<(), usize>,
    ) -> Result<()> {
        let mut after = start.map(|(key, seq)| (key.to_owned(), seq));
        let mut limit = first.max(1);
        loop {
            let asked = limit;
            let cursor = after.as_ref().map(|(key, seq)| (key.as_str(), *seq));
            let page = self.page(collection, cursor, unread, asked)?;
            let read = page.len();
            for item in page {
                after = Some((item.sort_key.clone(), item.seq));
                match visit(item) {
                    ControlFlow::Break(()) => return Ok(()),
                    ControlFlow::Continue(next) => limit = next.max(1),
                }
            }
            if read < asked {
                return Ok(());
            }
        }
    }

    /// One keyset page of [`Self::walk`], newest first below `after`.
    fn page(
        &self,
        collection: &str,
        after: Option<(&str, i64)>,
        unread: bool,
        limit: usize,
    ) -> Result<Vec<PimdirItem>> {
        if !unread {
            return self
                .store
                .list_summaries(collection, after, limit)
                .map_err(|err| anyhow!("List items in `{collection}`: {err}"));
        }

        let filter = PimdirMailFilter {
            seen: Some(false),
            attachment: None,
        };
        let cursor = after.map(|(key, seq)| PimdirMailCursor {
            sort_key: key.to_owned(),
            seq,
            collection: collection.to_owned(),
        });
        let entries = self
            .store
            .list_mail_page_filtered(&[collection], filter, cursor.as_ref(), limit)
            .map_err(|err| anyhow!("List unread items in `{collection}`: {err}"))?;
        Ok(entries.into_iter().map(|entry| entry.item).collect())
    }

    /// Whether no queued action changes what a read of `collection` shows,
    /// so its committed rows are what the overlaying reader returns: no
    /// `set-flags`, `update` or `remove` on it, no `move` out of it, no
    /// `move` or `copy` into it. A queue that does not read counts as
    /// changing it.
    fn overlay_quiet(&self, collection: &str) -> bool {
        let Ok(queued) = self.store.list_pending_actions() else {
            return false;
        };
        !queued.iter().any(|queued| {
            let here = queued.collection == collection;
            match &queued.action {
                PimdirAction::SetFlags { .. }
                | PimdirAction::Update { .. }
                | PimdirAction::Remove { .. } => here,
                PimdirAction::Move { to, .. } | PimdirAction::Copy { to, .. } => {
                    here || to.0 == collection
                }
                _ => false,
            }
        })
    }

    /// Pulls every live item of a collection with its summary by keyset
    /// paging, newest first (the read API is paginated; the shared
    /// list/search commands paginate in memory, as the file backends do).
    fn scan_items(&self, collection: &str) -> Result<Vec<PimdirItem>> {
        let mut all: Vec<PimdirItem> = Vec::new();
        loop {
            let after = all.last().map(|item| (item.sort_key.as_str(), item.seq));
            let page = self
                .store
                .list_summaries(collection, after, SCAN_BATCH)
                .map_err(|err| anyhow!("List items in `{collection}`: {err}"))?;
            let n = page.len();
            all.extend(page);
            if n < SCAN_BATCH {
                break;
            }
        }
        Ok(all)
    }

    /// The mailbox's pending creates and sends, in append order: the queued
    /// rows that are mail with no public id yet.
    fn queued_mail(&self, mailbox: &str) -> Result<Vec<PimdirPendingAction>> {
        let collection = self.hub_id(mailbox)?;
        let pending = self
            .store
            .pending_actions(&collection)
            .map_err(|err| anyhow!("List queued messages in `{mailbox}`: {err}"))?;

        Ok(pending
            .into_iter()
            .filter(|queued| is_queued_mail(&queued.action))
            .collect())
    }
}

/// The payload of a `submit` intent, version 1.
#[derive(Serialize)]
struct SubmitPayload<'a> {
    v: u8,
    object: &'a str,
    from: &'a str,
    rcpts: &'a [String],
    #[serde(skip_serializing_if = "Option::is_none")]
    subject: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    copy: Option<&'a str>,
}

/// Whether a queued action is a message waiting for a public id: a create,
/// or a send the owner performs.
fn is_queued_mail(action: &PimdirAction) -> bool {
    match action {
        PimdirAction::Add { .. } => true,
        PimdirAction::Unknown { kind, .. } => kind == SUBMIT,
        _ => false,
    }
}

/// Builds a shared [`Envelope`] from a stored item (no body read): the
/// flags from the item, the display fields from its mail summary.
fn envelope_from_item(item: &PimdirItem) -> Envelope {
    // NOTE: the public id is a short store-global integer rather than the
    // long link id.
    envelope(
        item.seq.to_string(),
        &item.flags,
        mail_summary(item.summary.as_ref()),
    )
}

/// The mail summary a read carries, none for an unfetched or non-mail item.
fn mail_summary(summary: Option<&PimdirSummary>) -> Option<&PimdirMailSummary> {
    match summary {
        Some(PimdirSummary::Mail(mail)) => Some(mail),
        _ => None,
    }
}

/// Builds an envelope from a flag set and a mail summary, `id` being the
/// public `seq`, or empty for a queued creation that has none yet.
fn envelope(id: String, flags: &PimdirFlags, summary: Option<&PimdirMailSummary>) -> Envelope {
    let flags = flags
        .known()
        .map(|flags| {
            flags
                .iter()
                .map(|raw| Flag::from_raw(raw.clone()))
                .collect()
        })
        .unwrap_or_default();
    let mail = summary.cloned().unwrap_or_default();

    Envelope {
        id,
        message_id: mail.message_id,
        in_reply_to: mail.in_reply_to,
        flags,
        subject: mail.subject,
        from: mail.from.into_iter().map(address).collect(),
        to: mail.to.into_iter().map(address).collect(),
        date: mail
            .date
            .as_deref()
            .and_then(|date| DateTime::parse_from_rfc3339(date).ok()),
        size: mail.size.unwrap_or(0),
        has_attachment: mail.attachment,
    }
}

/// A summary's canonical address as the shared one.
fn address(address: PimdirAddress) -> Address {
    Address {
        name: address.name,
        email: address.address,
    }
}

/// A mailbox with what the store says it holds of its server.
#[derive(Clone, Debug)]
pub struct PimdirCoveredMailbox {
    /// The mailbox, with both counts.
    pub mailbox: Mailbox,
    /// The scope its sources' last closed rounds covered and when, `None`
    /// before every source closed one.
    pub coverage: Option<PimdirCoverage>,
    /// The round a source has under way, if any.
    pub round: Option<PimdirRoundState>,
}

/// A message staged for the owner: the queue row it waits in, and the link
/// id it is filed under once applied.
#[derive(Clone, Debug)]
pub struct PimdirStaged {
    /// The queue row id, which `pimdir queue show` and `cancel` take.
    pub queue_id: i64,
    /// The bare `Message-ID` the message is stored under.
    pub message_id: String,
}

/// A message queued for sending.
#[derive(Clone, Debug)]
pub struct PimdirSubmitted {
    /// The `submit` row and the message's link id.
    pub staged: PimdirStaged,
    /// Whether the intent carries the copy, filed by the owner once sent.
    pub carried: bool,
}

/// A mailbox creation queued for the owner.
#[derive(Clone, Debug)]
pub struct PimdirCreated {
    /// The `collection-create` row.
    pub queue_id: i64,
    /// The source that creates the mailbox on its server.
    pub source: String,
}

/// One queued creation or send, as `pimdir queue list` shows it: the row an
/// operator acts on, plus the mail the action carries.
#[derive(Clone, Debug)]
pub struct PimdirQueued {
    /// The queue row id, which `pimdir queue cancel` takes.
    pub id: i64,
    /// The RFC 3339 instant the row was enqueued, stamped by the store.
    pub created_at: String,
    /// The process that staged it.
    pub producer: String,
    /// Whether the row sends the message rather than files it.
    pub send: bool,
    /// The message the action carries, derived from the body it pins. It has
    /// no `id`: a create has none until the owner applies it, a send none at
    /// all.
    pub envelope: Envelope,
}

/// Builds a queued row from a pending add or send and the body it pins,
/// skipping every other action.
///
/// The envelope keeps an empty id on purpose: the message has no public id
/// yet, and the queue row id belongs to another space than the field every
/// command reads back.
fn queued_from_action(queued: &PimdirPendingAction, body: Option<&[u8]>) -> Option<PimdirQueued> {
    // NOTE: a send is filed nowhere and carries no flags; it shows as read,
    // the way a sent copy is saved.
    let seen: PimdirFlags = [Flag::from_iana(IanaFlag::Seen)]
        .iter()
        .map(Flag::raw)
        .collect();
    let (send, flags) = match &queued.action {
        PimdirAction::Add { flags, .. } => (false, flags),
        action if is_queued_mail(action) => (true, &seen),
        _ => return None,
    };

    let summary = body.map(mail::derive).and_then(|derived| derived.summary);
    let envelope = envelope(String::new(), flags, mail_summary(summary.as_ref()));

    Some(PimdirQueued {
        id: queued.id,
        created_at: queued.created_at.clone(),
        producer: queued.producer.clone(),
        send,
        envelope,
    })
}

/// Streams `raw` into the blob store under `hash` and commits it durably,
/// returning the stored size.
fn write_blob(mut writer: PimdirBlobWriter, raw: &[u8], hash: &PimdirHash) -> Result<u64> {
    writer.write_all(raw)?;
    Ok(writer.commit(hash)?)
}

/// Applies a flag op to a base set, producing the replacement set `SetFlags`
/// stores. An unknown base holds no markers to build on, so it reads as empty.
fn apply_flag_op(current: &PimdirFlags, flags: &[Flag], op: FlagOp) -> PimdirFlags {
    let incoming = flags.iter().map(|flag| flag.raw().to_string());
    match op {
        FlagOp::Set => incoming.collect(),
        FlagOp::Add => {
            let mut set = current.known().cloned().unwrap_or_default();
            set.extend(incoming);
            PimdirFlags::Known(set)
        }
        FlagOp::Remove => {
            let mut set = current.known().cloned().unwrap_or_default();
            for flag in incoming {
                set.remove(&flag);
            }
            PimdirFlags::Known(set)
        }
    }
}

/// Parses a message id, the public `seq`, off the command line, with a clear
/// error for a non-numeric one.
fn parse_id(id: &str) -> Result<i64> {
    id.parse::<i64>()
        .map_err(|_| anyhow!("Invalid message id `{id}` (expected a number)"))
}

/// What a search query states that the store's reads express, every
/// clause still being matched in memory, so a bound only skips rows the
/// query cannot match.
///
/// Only clauses joined by `and` at the top of the query count. The bounds
/// are days (`YYYY-MM-DD`) compared with the sort key, which for mail is
/// the summary date as `YYYY-MM-DDTHH:MM:SSZ` (pimdir STORAGE §9.3, Annex
/// A.1), empty for an undated row: the key of an instant on a day sorts at
/// or above that day's own key and below the next one's. A date clause
/// reads the day in UTC, the offset the store keeps.
#[derive(Debug, Default, PartialEq)]
struct PimdirPushdown {
    /// The lowest sort key a match can have, inclusive: an undated row
    /// sorts below any.
    from: Option<String>,
    /// The sort key every match sorts below.
    before: Option<String>,
    /// Whether only unread mail matches: `not flag seen`.
    unread: bool,
}

impl PimdirPushdown {
    fn of(filter: &SearchEmailsFilterQuery) -> Self {
        let mut pushdown = Self::default();
        pushdown.add(filter);
        pushdown
    }

    fn add(&mut self, clause: &SearchEmailsFilterQuery) {
        use SearchEmailsFilterQuery as Q;

        match clause {
            Q::And(left, right) => {
                self.add(left);
                self.add(right);
            }
            Q::Date(day) => {
                self.raise(day_key(*day));
                self.lower(next_day_key(*day));
            }
            Q::AfterDate(day) => self.raise(next_day_key(*day)),
            Q::Not(inner) => match inner.as_ref() {
                // NOTE: an undated row is not after any day, so it matches
                // and the bound is an upper one only.
                Q::AfterDate(day) => self.lower(next_day_key(*day)),
                // NOTE: the chip reads `\Seen` in that one spelling, so it reads
                // every row unread here and some read in another spelling,
                // which the match drops.
                Q::Flag(flag) if flag.iana() == Some(IanaFlag::Seen) => self.unread = true,
                _ => {}
            },
            _ => {}
        }
    }

    fn raise(&mut self, key: Option<String>) {
        if let Some(key) = key
            && self.from.as_ref().is_none_or(|from| key > *from)
        {
            self.from = Some(key);
        }
    }

    fn lower(&mut self, key: Option<String>) {
        if let Some(key) = key
            && self.before.as_ref().is_none_or(|before| key < *before)
        {
            self.before = Some(key);
        }
    }
}

/// A day as the sort key it bounds, `None` for a year the key's four
/// digits cannot spell.
fn day_key(day: NaiveDate) -> Option<String> {
    (0..=9999)
        .contains(&day.year())
        .then(|| day.format("%Y-%m-%d").to_string())
}

/// The day after `day` as a sort key.
fn next_day_key(day: NaiveDate) -> Option<String> {
    day.succ_opt().and_then(day_key)
}

/// Whether a search order is the store's own, date descending: none, or
/// `date desc` alone, both sorting the undated last and keeping ties in
/// the order read.
fn is_store_order(sort: Option<&[SearchEmailsSorter]>) -> bool {
    sort.is_none_or(|chain| {
        chain.is_empty()
            || chain
                == [SearchEmailsSorter(
                    SearchEmailsSorterKind::Date,
                    SearchEmailsSorterOrder::Descending,
                )]
    })
}

/// Whether a row sits where its date files it, which the search bounds and
/// early stop rest on: its sort key is its summary date written as
/// `YYYY-MM-DDTHH:MM:SSZ`, or empty for a row whose date does not read and
/// so sorts last.
fn in_store_order(item: &PimdirItem) -> bool {
    let date = mail_summary(item.summary.as_ref())
        .and_then(|mail| mail.date.as_deref())
        .filter(|date| DateTime::parse_from_rfc3339(date).is_ok());
    match date {
        None => item.sort_key.is_empty(),
        Some(date) => {
            date == item.sort_key
                && NaiveDateTime::parse_from_str(date, KEY_FORMAT)
                    .is_ok_and(|at| at.format(KEY_FORMAT).to_string() == date)
        }
    }
}

/// Where a 1-indexed page ends in the full listing, `None` with no page
/// size, which asks for everything.
fn page_end(page: Option<u32>, page_size: Option<u32>) -> Option<usize> {
    let size = page_size? as usize;
    let page = page.unwrap_or(1).max(1) as usize;
    Some((page - 1).saturating_mul(size).saturating_add(size))
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

    use io_pimdir::placement::{PimdirLevel, PimdirLinkId};

    use super::*;
    use crate::config::PimdirConfig;

    /// A mailbox carries the role its server stated, which the sync engine
    /// recorded in the store (pimdir STORAGE §14), and none otherwise.
    #[test]
    fn a_mailbox_lists_the_role_its_store_records() {
        let dir = tempfile::tempdir().unwrap();
        {
            let store = io_pimdir::client::PimdirStore::open(dir.path())
                .unwrap()
                .for_account("work");
            for id in ["imap/INBOX", "imap/Sent Items", "imap/Work"] {
                store.ensure_collection(id, MAIL_KIND).unwrap();
            }
            store
                .set_collection_role("imap/INBOX", Some("inbox"))
                .unwrap();
            store
                .set_collection_role("imap/Sent Items", Some("sent"))
                .unwrap();
        }

        let mut client = PimdirClient::new(PimdirConfig {
            root: dir.path().to_path_buf(),
            account: None,
        })
        .unwrap();
        let roles: Vec<(String, Option<MailboxRole>)> = client
            .list_mailboxes(false)
            .unwrap()
            .into_iter()
            .map(|mailbox| (mailbox.id, mailbox.role))
            .collect();
        assert_eq!(
            roles,
            [
                ("imap/INBOX".into(), Some(MailboxRole::Inbox)),
                ("imap/Sent Items".into(), Some(MailboxRole::Sent)),
                ("imap/Work".into(), None),
            ]
        );
    }

    /// A mailbox carries the coverage its sources' closed rounds left and
    /// the round one has under way; one no round closed on carries none.
    #[test]
    fn a_mailbox_lists_its_coverage_and_round() {
        use io_pimdir::{
            change::PimdirWriteOp,
            collection::{PimdirCollectionId, PimdirScope},
        };

        let dir = tempfile::tempdir().unwrap();
        {
            let mut store = io_pimdir::client::PimdirStore::open(dir.path())
                .unwrap()
                .for_source("imap");
            for id in ["imap/INBOX", "imap/Work"] {
                store.ensure_collection(id, MAIL_KIND).unwrap();
            }
            let inbox = PimdirCollectionId("imap/INBOX".into());
            let work = PimdirCollectionId("imap/Work".into());
            store
                .write(vec![
                    PimdirWriteOp::OpenRound {
                        collection: inbox.clone(),
                        scope: PimdirScope::since("2026-09-07T00:00:00Z"),
                    },
                    PimdirWriteOp::CloseRound {
                        collection: inbox.clone(),
                        coverage: PimdirScope::since("2026-09-07T00:00:00Z"),
                        checkpoint: None,
                    },
                ])
                .unwrap();
            store
                .write(vec![PimdirWriteOp::OpenRound {
                    collection: inbox,
                    scope: PimdirScope::unbounded(),
                }])
                .unwrap();
            store
                .write(vec![PimdirWriteOp::OpenRound {
                    collection: work,
                    scope: PimdirScope::since("2026-09-07T00:00:00Z"),
                }])
                .unwrap();
        }

        let mut client = PimdirClient::new(PimdirConfig {
            root: dir.path().to_path_buf(),
            account: None,
        })
        .unwrap();
        let mailboxes = client.list_covered_mailboxes().unwrap();
        assert_eq!(mailboxes.len(), 2);

        let inbox = &mailboxes[0];
        assert_eq!(inbox.mailbox.id, "imap/INBOX");
        assert_eq!(
            (inbox.mailbox.total, inbox.mailbox.unread),
            (Some(0), Some(0))
        );
        let coverage = inbox.coverage.as_ref().expect("a closed round covers");
        assert_eq!(
            coverage.scope.since.as_deref(),
            Some("2026-09-07T00:00:00Z")
        );
        assert_eq!(coverage.scope.until, None);
        assert!(!coverage.at.is_empty());
        let round = inbox.round.as_ref().expect("a round is under way");
        assert_eq!(round.scope, PimdirScope::unbounded());

        let work = &mailboxes[1];
        assert_eq!(work.mailbox.id, "imap/Work");
        assert_eq!(work.coverage, None);
        assert_eq!(
            work.round
                .as_ref()
                .map(|round| round.scope.since.as_deref()),
            Some(Some("2026-09-07T00:00:00Z"))
        );
    }

    /// The attachment mark reads as stored: set, cleared, or not known.
    #[test]
    fn the_attachment_mark_reads_as_stored() {
        for (mark, json) in [
            (Some(true), serde_json::json!(true)),
            (Some(false), serde_json::json!(false)),
            (None, serde_json::Value::Null),
        ] {
            let envelope = envelope_from_item(&item(
                1,
                "x@y",
                known(&[]),
                PimdirMailSummary {
                    attachment: mark,
                    ..Default::default()
                },
            ));
            assert_eq!(envelope.has_attachment, mark);
            assert_eq!(
                serde_json::to_value(&envelope).unwrap()["has-attachment"],
                json
            );
        }
    }

    /// A stored item at the `Meta` tier: a mail summary and no body.
    fn item(seq: i64, link_id: &str, flags: PimdirFlags, mail: PimdirMailSummary) -> PimdirItem {
        PimdirItem {
            seq,
            link_id: PimdirLinkId(link_id.into()),
            flags,
            sort_key: String::new(),
            object: None,
            level: PimdirLevel::Meta,
            summary: Some(PimdirSummary::Mail(mail)),
            retention: None,
        }
    }

    fn known(flags: &[&str]) -> PimdirFlags {
        PimdirFlags::from_iter(flags.iter().copied())
    }

    fn person(address: &str, name: Option<&str>) -> PimdirAddress {
        PimdirAddress {
            address: address.into(),
            name: name.map(String::from),
        }
    }

    #[test]
    fn envelope_is_built_from_the_summary_without_a_body() {
        let item = item(
            42,
            "x@y",
            known(&["\\Seen"]),
            PimdirMailSummary {
                message_id: Some("x@y".into()),
                in_reply_to: vec!["parent@y".into()],
                subject: "Hi".into(),
                date: Some("2026-08-01T10:00:00Z".into()),
                size: Some(99),
                attachment: Some(true),
                from: vec![person("a@x.org", Some("Alice"))],
                to: vec![person("b@x.org", None)],
                ..Default::default()
            },
        );
        let envelope = envelope_from_item(&item);
        assert_eq!(envelope.id, "42");
        assert_eq!(envelope.message_id.as_deref(), Some("x@y"));
        assert_eq!(envelope.in_reply_to, ["parent@y"]);
        assert_eq!(envelope.subject, "Hi");
        assert_eq!(envelope.from[0].email, "a@x.org");
        assert_eq!(envelope.from[0].name.as_deref(), Some("Alice"));
        assert_eq!(envelope.to[0].email, "b@x.org");
        assert_eq!(envelope.to[0].name, None);
        assert_eq!(
            envelope.date.map(|date| date.to_rfc3339()).as_deref(),
            Some("2026-08-01T10:00:00+00:00")
        );
        assert_eq!(envelope.size, 99);
        assert_eq!(envelope.has_attachment, Some(true));
        assert!(envelope.flags.iter().any(|f| f.raw() == "\\Seen"));
    }

    /// A mailbox holding one `Message-ID` twice is ordinary, and the store
    /// keys the second copy apart under a minted one rather than keep one of
    /// the two.
    ///
    /// Both project as ordinary envelopes: the id a user sees is the `seq`,
    /// which differs between them, so neither hides the other and the minted
    /// key never shows.
    #[test]
    fn two_items_sharing_a_message_id_project_two_public_ids() {
        let twice = || PimdirMailSummary {
            message_id: Some("twice@host".into()),
            subject: "Twice".into(),
            ..Default::default()
        };

        let bare = envelope_from_item(&item(11, "twice@host", known(&[]), twice()));
        let minted = envelope_from_item(&item(12, "dup:twice@host#1174", known(&[]), twice()));

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
            link_id: PimdirLinkId("x@y".into()),
            flags: PimdirFlags::Unknown,
            sort_key: String::new(),
            object: None,
            level: PimdirLevel::Meta,
            summary: None,
            retention: None,
        };
        let envelope = envelope_from_item(&item);
        assert!(envelope.flags.is_empty());
        assert_eq!(envelope.id, "1");
        assert_eq!(envelope.subject, "");
    }

    #[test]
    fn flag_ops_add_set_and_remove() {
        let base = known(&["\\Seen"]);
        let flagged = [Flag::from_raw("\\Flagged")];
        let added = apply_flag_op(&base, &flagged, FlagOp::Add);
        assert!(added.contains("\\Seen") && added.contains("\\Flagged"));
        let set = apply_flag_op(&base, &flagged, FlagOp::Set);
        assert!(!set.contains("\\Seen") && set.contains("\\Flagged"));
        let seen = [Flag::from_raw("\\Seen")];
        let removed = apply_flag_op(&base, &seen, FlagOp::Remove);
        assert!(!removed.contains("\\Seen"));
    }

    /// An add onto an unknown set must not carry the unknown forward: the
    /// action replaces the set, and an unknown one erases what a sync knows.
    #[test]
    fn a_flag_op_on_an_unknown_set_stages_a_known_one() {
        let staged = apply_flag_op(
            &PimdirFlags::Unknown,
            &[Flag::from_raw("\\Seen")],
            FlagOp::Add,
        );
        assert_eq!(staged.known().map(BTreeSet::len), Some(1));
        assert!(staged.contains("\\Seen"));
    }

    /// A queued add carries no summary, so the row is derived from the body
    /// the action pins, the way the owner will derive it.
    #[test]
    fn a_queued_creation_renders_as_mail_with_no_id() {
        let queued = PimdirPendingAction {
            id: 7,
            created_at: "2026-08-27T10:00:00Z".into(),
            producer: "himalaya".into(),
            collection: "INBOX".into(),
            action: PimdirAction::Add {
                link_id: Some(PimdirLinkId("draft@x.org".into())),
                flags: known(&["\\Draft"]),
                object: Some(PimdirHash("cafe".into())),
            },
            attempts: 0,
        };
        let body = b"Message-ID: <draft@x.org>\r\nSubject: Re: lunch\r\nTo: Alice <alice@x.org>\r\n\r\nbody";

        let queued = queued_from_action(&queued, Some(body)).unwrap();

        assert_eq!(queued.id, 7);
        assert_eq!(queued.created_at, "2026-08-27T10:00:00Z");
        assert_eq!(queued.envelope.subject, "Re: lunch");
        assert_eq!(queued.envelope.to[0].email, "alice@x.org");
        assert_eq!(queued.envelope.message_id.as_deref(), Some("draft@x.org"));
        assert!(queued.envelope.flags.iter().any(|f| f.raw() == "\\Draft"));
        // NOTE: the row id names an action rather than a message, so it
        // belongs to another space than the field commands read back.
        assert!(queued.envelope.id.is_empty());
    }

    #[test]
    fn only_a_queued_creation_renders_as_mail() {
        let queued = PimdirPendingAction {
            id: 8,
            created_at: "2026-08-27T10:00:00Z".into(),
            producer: "himalaya".into(),
            collection: "INBOX".into(),
            action: PimdirAction::Remove { seq: 42 },
            attempts: 0,
        };

        // NOTE: a staged removal addresses a message that exists, so the
        // ordinary listing shows it and nothing is rendered here.
        assert!(queued_from_action(&queued, None).is_none());
    }

    /// A store holding one mail collection, `imap/Sent`, and the client
    /// reading it.
    fn sent_store() -> (tempfile::TempDir, PimdirClient) {
        let dir = tempfile::tempdir().unwrap();
        let store = io_pimdir::client::PimdirStore::open(dir.path()).unwrap();
        store.ensure_collection("imap/Sent", MAIL_KIND).unwrap();
        drop(store);

        let config = crate::config::PimdirConfig {
            root: dir.path().to_path_buf(),
            account: None,
        };
        (dir, PimdirClient::new(config).unwrap())
    }

    const RAW: &[u8] = b"From: a@x.org\r\nTo: b@y.org\r\nBcc: c@y.org\r\nSubject: hi\r\n\r\nhello";

    #[test]
    fn a_sent_message_is_one_submit_row_with_its_envelope() {
        let (_dir, mut client) = sent_store();

        client
            .send_message(Some("imap/Sent"), RAW.to_vec(), false)
            .unwrap();

        let pending = client.store.pending_actions("imap/Sent").unwrap();
        assert_eq!(pending.len(), 1);
        let PimdirAction::Unknown {
            kind,
            payload,
            object_hash,
        } = &pending[0].action
        else {
            panic!("expected a submit intent, got {:?}", pending[0].action);
        };
        assert_eq!(kind, SUBMIT);

        // NOTE: the `submit` payload a store owner decodes, `object` being
        // the pinned body by the shared queue convention.
        let hash = object_hash.as_ref().unwrap();
        let payload: serde_json::Value = serde_json::from_str(payload).unwrap();
        assert_eq!(
            payload,
            serde_json::json!({
                "v": 1,
                "object": hash.0,
                "from": "a@x.org",
                "rcpts": ["b@y.org", "c@y.org"],
                "subject": "hi",
            })
        );

        // NOTE: the body keeps its Bcc field, which the sending channel
        // removes; Graph derives its recipients from it.
        assert_eq!(client.blobs.get(hash).unwrap().as_deref(), Some(RAW));

        let queued = client.queued_envelopes("imap/Sent").unwrap();
        assert_eq!(queued.len(), 1);
        assert!(queued[0].send);
        assert_eq!(queued[0].envelope.subject, "hi");
    }

    #[test]
    fn a_send_asking_for_a_copy_leaves_it_to_the_caller_on_an_undeclared_owner() {
        let (_dir, mut client) = sent_store();

        let carried = client
            .send_message(Some("imap/Sent"), RAW.to_vec(), true)
            .unwrap();

        // NOTE: an owner predating capabilities would ignore `copy`.
        assert!(!carried.carried);
        let pending = client.store.pending_actions("imap/Sent").unwrap();
        let PimdirAction::Unknown { payload, .. } = &pending[0].action else {
            panic!("expected a submit intent, got {:?}", pending[0].action);
        };
        let payload: serde_json::Value = serde_json::from_str(payload).unwrap();
        assert!(payload.get("copy").is_none());
    }

    #[test]
    fn a_send_asking_for_a_copy_carries_it_in_the_intent() {
        let (dir, mut client) = sent_store();
        declare(
            dir.path(),
            &[capability::MAIL_SUBMIT, capability::MAIL_SUBMIT_COPY],
        );

        let carried = client
            .send_message(Some("imap/Sent"), RAW.to_vec(), true)
            .unwrap();
        assert!(carried.carried);

        let pending = client.store.pending_actions("imap/Sent").unwrap();
        assert_eq!(pending.len(), 1, "the copy rides on the submit, not an add");
        let PimdirAction::Unknown { payload, .. } = &pending[0].action else {
            panic!("expected a submit intent, got {:?}", pending[0].action);
        };
        let payload: serde_json::Value = serde_json::from_str(payload).unwrap();
        assert_eq!(payload["copy"], "imap/Sent");
    }

    #[test]
    fn a_send_with_no_mailbox_stages_nothing() {
        let (_dir, mut client) = sent_store();

        let err = client.send_message(None, RAW.to_vec(), false).unwrap_err();

        assert!(err.to_string().contains("mailbox.alias.sent"));
        assert!(
            client
                .store
                .pending_actions("imap/Sent")
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn a_send_with_no_recipient_stages_nothing() {
        let (_dir, mut client) = sent_store();
        let raw = b"From: a@x.org\r\nSubject: hi\r\n\r\nhello".to_vec();

        assert!(client.send_message(Some("imap/Sent"), raw, false).is_err());
        assert!(
            client
                .store
                .pending_actions("imap/Sent")
                .unwrap()
                .is_empty()
        );
    }

    const DRAFT: &[u8] =
        b"Message-ID: <d1@x.org>\r\nFrom: a@x.org\r\nTo: b@y.org\r\nSubject: hi\r\n\r\nhello";

    #[test]
    fn an_added_message_names_its_queue_row_and_message_id() {
        let (_dir, mut client) = sent_store();

        let staged = client
            .add_message("imap/Sent", &[], DRAFT.to_vec())
            .unwrap();

        assert_eq!(staged.message_id, "d1@x.org");
        let pending = client.store.pending_actions("imap/Sent").unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, staged.queue_id);
    }

    #[test]
    fn a_sent_message_names_its_queue_row_and_message_id() {
        let (_dir, mut client) = sent_store();

        let sent = client
            .send_message(Some("imap/Sent"), DRAFT.to_vec(), false)
            .unwrap();

        assert_eq!(sent.staged.message_id, "d1@x.org");
        let pending = client.store.pending_actions("imap/Sent").unwrap();
        assert_eq!(pending[0].id, sent.staged.queue_id);
    }

    #[test]
    fn a_queue_row_is_pending_then_unknown_once_cancelled() {
        let (_dir, mut client) = sent_store();
        let staged = client
            .add_message("imap/Sent", &[], DRAFT.to_vec())
            .unwrap();

        let PimdirActionStatus::Pending {
            collection, kind, ..
        } = client.queue_row(staged.queue_id).unwrap()
        else {
            panic!("expected a pending row");
        };
        assert_eq!(collection, "imap/Sent");
        assert_eq!(kind, "add");

        assert!(client.cancel_queued(staged.queue_id).unwrap());
        assert_eq!(
            client.queue_row(staged.queue_id).unwrap(),
            PimdirActionStatus::Unknown
        );
    }

    #[test]
    fn a_sent_message_reads_applied_once_its_sender_acknowledges_it() {
        let (dir, mut client) = sent_store();
        let sent = client
            .send_message(Some("imap/Sent"), DRAFT.to_vec(), false)
            .unwrap();

        // NOTE: what the sync engine does once the message has left (pimdir
        // STORAGE §15.5): the intent goes, its receipt stays.
        let mut owner = io_pimdir::client::PimdirStore::open(dir.path()).unwrap();
        assert!(
            owner
                .acknowledge_action(sent.staged.queue_id, None)
                .unwrap()
        );
        drop(owner);

        let PimdirActionStatus::Applied {
            collection, seq, ..
        } = client.queue_row(sent.staged.queue_id).unwrap()
        else {
            panic!("expected a sent row to read applied");
        };
        assert_eq!(collection, "imap/Sent");
        assert_eq!(seq, None);
    }

    #[test]
    fn an_applied_add_names_the_seq_it_created() {
        let (dir, mut client) = sent_store();
        let staged = client
            .add_message("imap/Sent", &[], DRAFT.to_vec())
            .unwrap();

        let mut owner = io_pimdir::client::PimdirStore::open(dir.path())
            .unwrap()
            .for_source("imap");
        assert_eq!(owner.drain().unwrap().applied, 1);
        drop(owner);

        let PimdirActionStatus::Applied {
            collection, seq, ..
        } = client.queue_row(staged.queue_id).unwrap()
        else {
            panic!("expected an applied row");
        };
        assert_eq!(collection, "imap/Sent");
        let seq = seq.expect("an add names the item it created");
        let item = client.get("imap/Sent", &seq.to_string()).unwrap().unwrap();
        assert_eq!(item.seq, seq);
    }

    /// Declares `names` for the `imap` source syncing `imap/Sent`.
    fn declare(dir: &std::path::Path, names: &[&str]) {
        let mut store = io_pimdir::client::PimdirStore::open(dir)
            .unwrap()
            .for_source("imap");
        store
            .write(vec![io_pimdir::change::PimdirWriteOp::SetCheckpoint {
                collection: io_pimdir::collection::PimdirCollectionId("imap/Sent".into()),
                checkpoint: io_pimdir::collection::PimdirCheckpoint(Vec::new()),
            }])
            .unwrap();
        let declaration: Vec<_> = names
            .iter()
            .map(|name| io_pimdir::capability::PimdirCapability {
                collection: None,
                name: name.to_string(),
                support: io_pimdir::capability::PimdirSupport::Full,
                detail: None,
            })
            .collect();
        store.declare("imap", &declaration).unwrap();
    }

    #[test]
    fn a_mailbox_creation_is_one_intent_naming_its_performer() {
        let (dir, mut client) = sent_store();
        declare(dir.path(), &[capability::COLLECTION_CREATE]);

        let created = client.create_mailbox("Projets", Some("imap/Sent")).unwrap();

        assert_eq!(created.source, "imap");
        let pending = client.store.pending_actions("imap/Sent").unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, created.queue_id);
        let PimdirAction::Unknown { kind, payload, .. } = &pending[0].action else {
            panic!("expected an intent, got {:?}", pending[0].action);
        };
        assert_eq!(kind, "collection-create");
        let payload: serde_json::Value = serde_json::from_str(payload).unwrap();
        assert_eq!(
            payload,
            serde_json::json!({
                "v": 1,
                "source": "imap",
                "name": "Projets",
                "parent": "imap/Sent",
            })
        );
    }

    #[test]
    fn a_mailbox_creation_at_the_top_has_no_parent() {
        let (dir, mut client) = sent_store();
        declare(dir.path(), &[capability::COLLECTION_CREATE]);

        client.create_mailbox("Projets", None).unwrap();

        let pending = client.store.pending_actions("imap/Sent").unwrap();
        let PimdirAction::Unknown { payload, .. } = &pending[0].action else {
            panic!("expected an intent, got {:?}", pending[0].action);
        };
        let payload: serde_json::Value = serde_json::from_str(payload).unwrap();
        assert!(payload.get("parent").is_none());
    }

    #[test]
    fn a_mailbox_creation_no_source_performs_stages_nothing() {
        let (dir, mut client) = sent_store();

        // NOTE: an undeclared owner predates the intent.
        assert!(client.create_mailbox("Projets", None).is_err());

        declare(dir.path(), &[capability::MAIL_SUBMIT]);
        assert!(client.create_mailbox("Projets", None).is_err());
        assert!(client.create_mailbox("", None).is_err());
        assert!(
            client
                .store
                .pending_actions("imap/Sent")
                .unwrap()
                .is_empty()
        );
    }

    /// One mail a seeded mailbox holds: its handle, its flags, and its
    /// summary, `None` for a row whose summary was never read.
    struct Seeded {
        handle: String,
        flags: PimdirFlags,
        summary: Option<PimdirMailSummary>,
    }

    /// `n` mails spread over six days of May 2026, with ties on the minute,
    /// undated ones, rows with no summary, read and unread (`\Seen` in two
    /// spellings), flagged, unknown flag sets, two senders on some, and an
    /// attachment mark set, cleared or absent.
    fn mails(prefix: &str, n: usize) -> Vec<Seeded> {
        let words = ["release", "Invoice", "météo", "hello"];
        (0..n)
            .map(|i| {
                let date = (i % 17 != 0).then(|| {
                    format!(
                        "2026-05-{:02}T{:02}:{:02}:00Z",
                        10 + (i * 7) % 6,
                        (i * 13) % 24,
                        i % 2,
                    )
                });
                let mut flags = Vec::new();
                if i % 3 == 0 {
                    flags.push("\\Seen");
                }
                if i % 5 == 0 {
                    flags.push("\\seen");
                }
                if i % 7 == 0 {
                    flags.push("\\Flagged");
                }
                let flags = match i % 11 {
                    0 => PimdirFlags::Unknown,
                    _ => known(&flags),
                };
                let mut from = vec![person(
                    &format!("user{}@example.org", i % 5),
                    (i % 2 == 0).then_some("Alice Martin"),
                )];
                if i % 4 == 0 {
                    from.push(person("second@example.org", Some("Second")));
                }
                let summary = (i % 23 != 0).then(|| PimdirMailSummary {
                    message_id: Some(format!("{prefix}{i}@x.test")),
                    subject: format!("{} {i}", words[i % words.len()]),
                    sender: from.first().map(|from| from.address.clone()),
                    sender_name: from.first().and_then(|from| from.name.clone()),
                    date,
                    size: Some(100 + i as u64),
                    attachment: match i % 9 {
                        0 => None,
                        _ => Some(i % 2 == 0),
                    },
                    from,
                    to: vec![person("team@example.org", None)],
                    ..Default::default()
                });
                Seeded {
                    handle: format!("{prefix}{i}"),
                    flags,
                    summary,
                }
            })
            .collect()
    }

    /// Files `mails` in `collection` as the `imap` source lists them, at the
    /// `Meta` tier, in batches.
    fn seed(dir: &std::path::Path, collection: &str, mails: &[Seeded]) {
        use io_pimdir::{
            change::PimdirWriteOp,
            placement::{PimdirBase, PimdirHandle, PimdirPlacement, PimdirStatus},
        };

        let mut store = io_pimdir::client::PimdirStore::open(dir)
            .unwrap()
            .for_source("imap");
        store.ensure_collection(collection, MAIL_KIND).unwrap();
        for batch in mails.chunks(1000) {
            let ops = batch
                .iter()
                .map(|mail| {
                    let (link_id, sort_key) = match &mail.summary {
                        Some(summary) => (summary.link_id(), summary.sort_key()),
                        None => (
                            PimdirLinkId(format!("{}@x.test", mail.handle)),
                            Default::default(),
                        ),
                    };
                    PimdirWriteOp::UpsertPlacement(PimdirPlacement {
                        collection: PimdirCollectionId(collection.into()),
                        handle: PimdirHandle(mail.handle.clone()),
                        link_id: Some(link_id),
                        object: None,
                        level: PimdirLevel::Meta,
                        summary: mail.summary.clone().map(PimdirSummary::Mail),
                        sort_key,
                        flags: mail.flags.clone(),
                        status: PimdirStatus::Clean,
                        conflict_revision: None,
                        conflict_object: None,
                        base: Some(PimdirBase {
                            flags: mail.flags.clone(),
                            revision: None,
                            object: None,
                        }),
                        origin: None,
                    })
                })
                .collect();
            store.write(ops).unwrap();
        }
    }

    /// A store holding `imap/INBOX` (300 mails) and `imap/Archive` (20), and
    /// the client reading it.
    fn seeded_store() -> (tempfile::TempDir, PimdirClient) {
        let dir = tempfile::tempdir().unwrap();
        seed(dir.path(), "imap/INBOX", &mails("in", 300));
        seed(dir.path(), "imap/Archive", &mails("ar", 20));
        let client = PimdirClient::new(PimdirConfig {
            root: dir.path().to_path_buf(),
            account: None,
        })
        .unwrap();
        (dir, client)
    }

    const QUERIES: &[&str] = &[
        "",
        "flag seen",
        "not flag seen",
        "not flag \\\\Seen",
        "flag flagged",
        "not flag flagged",
        "after 2026-05-12",
        "date 2026-05-13",
        "not after 2026-05-12",
        "after 2026-05-11 and not after 2026-05-13",
        "date 2026-05-13 and not flag seen",
        "not flag seen and after 2026-05-14",
        "date 2026-05-10 and date 2026-05-11",
        "after 2026-05-30",
        "after 1999-01-01",
        "not after 1999-01-01",
        "not date 2026-05-13",
        "not (after 2026-05-12 and flag seen)",
        "from user1",
        "from SECOND",
        "from alice and not flag seen",
        "subject release",
        "subject MÉTÉO",
        "to team",
        "body hello",
        "from user1 or flag seen",
        "order by date desc",
        "order by date",
        "order by from",
        "date 2026-05-13 order by date asc",
        "not flag seen order by subject desc",
        "after 2026-05-12 order by date desc",
    ];

    const PAGES: &[(Option<u32>, Option<u32>)] = &[
        (None, None),
        (Some(1), None),
        (Some(1), Some(10)),
        (Some(2), Some(10)),
        (Some(3), Some(7)),
        (Some(1), Some(1)),
        (None, Some(64)),
        (Some(2), Some(65)),
        (Some(5), Some(60)),
        (Some(50), Some(10)),
        (Some(1), Some(0)),
        (Some(1), Some(1000)),
        (Some(u32::MAX), Some(u32::MAX)),
    ];

    /// Every page of every query the paged reads answer as the whole-mailbox
    /// read did, byte for byte, through the pushed-down path.
    fn assert_paged_reads_agree(client: &mut PimdirClient, mailbox: &str) {
        for (page, size) in PAGES {
            let scanned: Vec<Envelope> = client
                .scan_items(mailbox)
                .unwrap()
                .iter()
                .map(envelope_from_item)
                .collect();
            let listed = client.list_envelopes(mailbox, *page, *size, false).unwrap();
            assert_eq!(
                serde_json::to_string(&listed).unwrap(),
                serde_json::to_string(&paginate(scanned, *page, *size)).unwrap(),
                "list page {page:?} size {size:?}",
            );

            for query in QUERIES {
                let parsed =
                    (!query.is_empty()).then(|| query.parse::<SearchEmailsQuery>().unwrap());
                let filter = parsed.as_ref().and_then(|q| q.filter.as_ref());
                let sort = parsed.as_ref().and_then(|q| q.sort.as_deref());
                let scanned = client
                    .search_scanned(mailbox, filter, sort, *page, *size)
                    .unwrap();
                let pushed = client
                    .search_pushed(mailbox, filter, sort, *page, *size)
                    .unwrap()
                    .expect("the seeded rows are filed under their dates");
                let searched = client
                    .search_envelopes(mailbox, parsed.as_ref(), *page, *size, false)
                    .unwrap();
                let scanned = serde_json::to_string(&scanned).unwrap();
                assert_eq!(
                    serde_json::to_string(&pushed).unwrap(),
                    scanned,
                    "search `{query}` page {page:?} size {size:?}",
                );
                assert_eq!(serde_json::to_string(&searched).unwrap(), scanned);
            }
        }
    }

    #[test]
    fn paged_reads_agree_with_the_whole_mailbox_read() {
        let (_dir, mut client) = seeded_store();
        assert!(client.overlay_quiet("imap/INBOX"));
        assert_paged_reads_agree(&mut client, "imap/INBOX");

        // NOTE: a sanity check that the seed holds what the queries reach.
        let unread = "not flag seen".parse::<SearchEmailsQuery>().unwrap();
        let hits = client
            .search_envelopes("imap/INBOX", Some(&unread), None, None, false)
            .unwrap();
        assert!(hits.len() > 50 && hits.len() < 250, "{}", hits.len());
        let undated = client
            .list_envelopes("imap/INBOX", None, None, false)
            .unwrap()
            .into_iter()
            .filter(|envelope| envelope.date.is_none())
            .count();
        assert!(undated > 10, "{undated}");
    }

    #[test]
    fn paged_reads_agree_with_the_whole_mailbox_read_over_a_busy_queue() {
        let (_dir, mut client) = seeded_store();
        let inbox: Vec<String> = client
            .list_envelopes("imap/INBOX", None, None, false)
            .unwrap()
            .into_iter()
            .map(|envelope| envelope.id)
            .collect();
        let archive: Vec<String> = client
            .list_envelopes("imap/Archive", None, None, false)
            .unwrap()
            .into_iter()
            .map(|envelope| envelope.id)
            .collect();
        let ids = |ids: &[String], step: usize, skip: usize| -> Vec<String> {
            ids.iter().skip(skip).step_by(step).cloned().collect()
        };
        fn refs(ids: &[String]) -> Vec<&str> {
            ids.iter().map(String::as_str).collect()
        }

        let seen = [Flag::from_iana(IanaFlag::Seen)];
        let read = ids(&inbox, 4, 1);
        client
            .store_flags("imap/INBOX", &refs(&read), &seen, FlagOp::Add)
            .unwrap();
        let unread = ids(&inbox, 6, 0);
        client
            .store_flags("imap/INBOX", &refs(&unread), &seen, FlagOp::Remove)
            .unwrap();
        let gone = ids(&inbox, 10, 3);
        client.delete_messages("imap/INBOX", &refs(&gone)).unwrap();
        let moved = ids(&inbox, 15, 2);
        client
            .move_messages("imap/INBOX", "imap/Archive", &refs(&moved))
            .unwrap();
        let arrived = ids(&archive, 2, 0);
        client
            .move_messages("imap/Archive", "imap/INBOX", &refs(&arrived))
            .unwrap();
        client.add_message("imap/INBOX", &[], RAW.to_vec()).unwrap();

        assert!(!client.overlay_quiet("imap/INBOX"));
        assert!(!client.overlay_quiet("imap/Archive"));
        assert_paged_reads_agree(&mut client, "imap/INBOX");
        assert_paged_reads_agree(&mut client, "imap/Archive");
    }

    #[test]
    fn a_queue_touching_another_mailbox_leaves_one_quiet() {
        let (dir, mut client) = seeded_store();
        seed(dir.path(), "imap/Work", &mails("wo", 5));
        let id = client
            .list_envelopes("imap/Archive", Some(1), Some(1), false)
            .unwrap()[0]
            .id
            .clone();
        client
            .store_flags(
                "imap/Archive",
                &[id.as_str()],
                &[Flag::from_iana(IanaFlag::Seen)],
                FlagOp::Add,
            )
            .unwrap();
        client.add_message("imap/INBOX", &[], RAW.to_vec()).unwrap();
        assert!(client.overlay_quiet("imap/INBOX"));
        assert!(!client.overlay_quiet("imap/Archive"));

        client
            .copy_messages("imap/Archive", "imap/Work", &[id.as_str()])
            .unwrap();
        assert!(!client.overlay_quiet("imap/Work"));
        assert!(client.overlay_quiet("imap/INBOX"));
    }

    /// A row filed under another key than its date sends the search back
    /// to the whole-mailbox read, which answers as before.
    #[test]
    fn a_row_out_of_date_order_searches_the_whole_mailbox() {
        let dir = tempfile::tempdir().unwrap();
        let mut seeded = mails("in", 40);
        let summary = seeded[1].summary.as_mut().unwrap();
        summary.date = Some("2026-05-12T10:00:00+02:00".into());
        seed(dir.path(), "imap/INBOX", &seeded);
        let mut client = PimdirClient::new(PimdirConfig {
            root: dir.path().to_path_buf(),
            account: None,
        })
        .unwrap();

        for query in ["", "after 2026-05-11", "not flag seen order by from"] {
            let parsed = (!query.is_empty()).then(|| query.parse::<SearchEmailsQuery>().unwrap());
            let filter = parsed.as_ref().and_then(|q| q.filter.as_ref());
            let sort = parsed.as_ref().and_then(|q| q.sort.as_deref());
            assert_eq!(
                client
                    .search_pushed("imap/INBOX", filter, sort, None, None)
                    .unwrap(),
                None,
                "`{query}`",
            );
            let scanned = client
                .search_scanned("imap/INBOX", filter, sort, Some(1), Some(10))
                .unwrap();
            let searched = client
                .search_envelopes("imap/INBOX", parsed.as_ref(), Some(1), Some(10), false)
                .unwrap();
            assert_eq!(
                serde_json::to_string(&searched).unwrap(),
                serde_json::to_string(&scanned).unwrap(),
            );
        }
    }

    #[test]
    fn date_clauses_bound_the_walk_on_the_sort_key() {
        let pushdown = |query: &str| {
            let query = query.parse::<SearchEmailsQuery>().unwrap();
            PimdirPushdown::of(query.filter.as_ref().unwrap())
        };
        let day = |day: &str| Some(day.to_string());

        assert_eq!(
            pushdown("date 2026-05-13 and not flag seen"),
            PimdirPushdown {
                from: day("2026-05-13"),
                before: day("2026-05-14"),
                unread: true,
            }
        );
        assert_eq!(
            pushdown("after 2026-05-11 and not after 2026-05-13 and after 2026-05-10"),
            PimdirPushdown {
                from: day("2026-05-12"),
                before: day("2026-05-14"),
                unread: false,
            }
        );
        assert_eq!(pushdown("after 2026-12-31").from, day("2027-01-01"));
        // NOTE: only clauses joined by `and` at the top bound the walk.
        assert_eq!(
            pushdown("after 2026-05-11 or flag seen"),
            PimdirPushdown::default()
        );
        assert_eq!(
            pushdown("not (after 2026-05-11 and not flag seen)"),
            PimdirPushdown::default()
        );
        assert_eq!(pushdown("flag seen and from x"), PimdirPushdown::default());
        assert_eq!(pushdown("after 9999-12-31"), PimdirPushdown::default());
    }

    #[test]
    fn a_page_ends_where_its_last_row_sits() {
        assert_eq!(page_end(None, None), None);
        assert_eq!(page_end(Some(3), None), None);
        assert_eq!(page_end(None, Some(50)), Some(50));
        assert_eq!(page_end(Some(0), Some(50)), Some(50));
        assert_eq!(page_end(Some(3), Some(50)), Some(150));
        assert_eq!(
            page_end(Some(u32::MAX), Some(u32::MAX)),
            Some(u32::MAX as usize * u32::MAX as usize)
        );
    }

    /// `envelope list -s 50` and a few searches on a mailbox of 50,000
    /// mails, read whole then paged; prints the timings.
    ///
    /// `cargo test --release --no-default-features --features pimdir,rustls-ring
    /// -- --ignored --nocapture fifty_thousand`
    #[test]
    #[ignore = "seeds 50,000 mails"]
    fn fifty_thousand_mails_list_one_page() {
        use std::time::{Duration, Instant};

        let dir = tempfile::tempdir().unwrap();
        let seeded = Instant::now();
        let mut many = mails("in", 50_000);
        for (i, mail) in many.iter_mut().enumerate() {
            if let Some(summary) = mail.summary.as_mut()
                && summary.date.is_some()
            {
                let at = 1_767_225_600 + (i as i64) * 600;
                let at = DateTime::from_timestamp(at, 0).unwrap();
                summary.date = Some(at.format(KEY_FORMAT).to_string());
            }
        }
        seed(dir.path(), "imap/INBOX", &many);
        eprintln!("seeded 50,000 mails in {:?}", seeded.elapsed());
        let mut client = PimdirClient::new(PimdirConfig {
            root: dir.path().to_path_buf(),
            account: None,
        })
        .unwrap();

        fn median(mut run: impl FnMut() -> usize) -> Duration {
            let mut times: Vec<Duration> = (0..5)
                .map(|_| {
                    let at = Instant::now();
                    std::hint::black_box(run());
                    at.elapsed()
                })
                .collect();
            times.sort();
            times[2]
        }

        let whole = median(|| {
            let all: Vec<Envelope> = client
                .scan_items("imap/INBOX")
                .unwrap()
                .iter()
                .map(envelope_from_item)
                .collect();
            paginate(all, Some(1), Some(50)).len()
        });
        eprintln!("envelope list -s 50, whole mailbox read: {whole:?}");
        for page in [1, 10, 100] {
            let paged = median(|| {
                client
                    .list_envelopes("imap/INBOX", Some(page), Some(50), false)
                    .unwrap()
                    .len()
            });
            eprintln!("envelope list -s 50 -p {page}, paged read: {paged:?}");
        }
        for query in [
            "not flag seen",
            "after 2026-08-01",
            "from user1",
            "subject release order by from",
        ] {
            let parsed = query.parse::<SearchEmailsQuery>().unwrap();
            let filter = parsed.filter.as_ref();
            let sort = parsed.sort.as_deref();
            let scanned = median(|| {
                client
                    .search_scanned("imap/INBOX", filter, sort, Some(1), Some(50))
                    .unwrap()
                    .len()
            });
            let pushed = median(|| {
                client
                    .search_envelopes("imap/INBOX", Some(&parsed), Some(1), Some(50), false)
                    .unwrap()
                    .len()
            });
            eprintln!("envelope search -s 50 `{query}`: whole {scanned:?}, paged {pushed:?}");
        }
    }
}
