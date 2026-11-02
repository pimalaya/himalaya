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

use std::{collections::BTreeSet, ops::ControlFlow};

use anyhow::{Context, Result, anyhow, bail};
use chrono::{DateTime, Datelike, FixedOffset, NaiveDate, NaiveDateTime};
use io_pimdir::{
    capability,
    client::{
        PimdirError,
        producer::{PimdirActionStatus, PimdirParkedAction, PimdirPendingAction},
        reader::{
            PimdirCollection, PimdirItem, PimdirMailCursor, PimdirMailFilter, PimdirRoundState,
        },
    },
    codec::PimdirAction,
    collection::{PimdirCollectionId, PimdirCoverage},
    intent::{self, PimdirSubmit},
    placement::PimdirFlags,
    summary::{
        PimdirAddress, PimdirSummary,
        mail::{self, PimdirMailSummary},
    },
};
use log::{info, warn};
use pimalaya_cli::error::CodedError;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    email::{
        address::Address,
        envelope::Envelope,
        flag::{Flag, FlagOp, IanaFlag},
        mailbox::{Mailbox, MailboxIndex, MailboxRole},
        search::{
            eval::{self, EnvelopeCopies},
            filter::query::SearchEmailsFilterQuery,
            query::SearchEmailsQuery,
            sort::query::{SearchEmailsSorter, SearchEmailsSorterKind, SearchEmailsSorterOrder},
        },
        submission::SubmissionEnvelope,
    },
    error::{ErrorCode, pimdir_error},
    pimdir::client::PimdirClient,
};

/// The mail media type a pimdir collection carries to be a mailbox.
const MAIL_KIND: &str = "message/rfc822";

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
    /// The collection a user-typed mailbox addresses: the mailbox itself,
    /// a pimdir mailbox being its collection id verbatim, else the mailbox
    /// carrying the role it names.
    ///
    /// The id is checked against the account's mail collections rather than
    /// passed through, an id nothing was written under otherwise reading as
    /// a mailbox that exists and is empty.
    pub(crate) fn hub_id(&self, mailbox: &str) -> Result<String> {
        let resolved = self.resolve_mailbox_id(mailbox)?;
        let mut ids: Vec<String> = self
            .mail_collections()?
            .into_iter()
            .map(|collection| collection.id)
            .collect();

        if ids.contains(&resolved) {
            return Ok(resolved);
        }

        ids.sort();

        bail!(
            "Mailbox `{mailbox}` not found in the pimdir store, which holds: {}",
            ids.join(", "),
        )
    }

    /// Maps a mailbox onto its collection id: an id of the account passes
    /// through, then a role name resolves to the mailbox the store records
    /// it on, erroring when several do. Anything else goes back as it is.
    ///
    /// A collection's name is not matched: two sources may each hold a
    /// mailbox of one name.
    pub fn resolve_mailbox_id(&self, mailbox: &str) -> Result<String> {
        let index = self.mailbox_index()?;
        if index.0.iter().any(|known| known.id == mailbox) {
            return Ok(mailbox.to_string());
        }

        if let Some(role) = MailboxRole::known(mailbox)
            && let Some(id) = index.with_role(&role)?
        {
            return Ok(id);
        }

        Ok(mailbox.to_string())
    }

    /// The id of the mailbox the store records `role` on, `None` when none
    /// carries it.
    pub fn role_mailbox_id(&self, role: &MailboxRole) -> Result<Option<String>> {
        self.mailbox_index()?.with_role(role)
    }

    /// The account's mailboxes with the roles the store records, no counts.
    fn mailbox_index(&self) -> Result<MailboxIndex> {
        let mailboxes = self
            .mail_collections()?
            .into_iter()
            .map(|collection| Mailbox {
                role: collection.role.as_deref().map(MailboxRole::parse),
                id: collection.id,
                name: collection.name,
                total: None,
                unread: None,
            })
            .collect();
        Ok(MailboxIndex(mailboxes))
    }

    /// The account's mail collections, in store order.
    fn mail_collections(&self) -> Result<Vec<PimdirCollection>> {
        Ok(self
            .store
            .list_collections_by_account(self.account.as_deref())
            .map_err(pimdir_error)
            .context("List pimdir collections")?
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
                        .map_err(pimdir_error)
                        .with_context(|| format!("Count items in `{}`", collection.id))?,
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
            .count_unread(&ids, None, None)
            .map_err(pimdir_error)
            .context("Count unread pimdir mail")?;

        let mut mailboxes = Vec::new();
        for collection in collections {
            let total = self
                .store
                .count_items(&collection.id)
                .map_err(pimdir_error)
                .with_context(|| format!("Count items in `{}`", collection.id))?;
            let round = self
                .store
                .list_coverage(&collection.id)
                .map_err(pimdir_error)
                .with_context(|| format!("Read the coverage of `{}`", collection.id))?
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
    /// applies the shared filter/sort/paginate. A query holding a body clause
    /// reads each candidate's local body, and a message whose body is not
    /// local matches no body clause; the other clauses read no body.
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
        let mut failed = None;
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
            match self.matches(&item, &envelope, filter) {
                Ok(true) => hits.push(envelope),
                Ok(false) => {}
                Err(err) => {
                    failed = Some(err);
                    return ControlFlow::Break(());
                }
            }
            let left = wanted.map(|wanted| wanted.saturating_sub(hits.len()));
            match left {
                Some(0) => ControlFlow::Break(()),
                left => ControlFlow::Continue(batch(left)),
            }
        })?;

        if let Some(err) = failed {
            return Err(err);
        }
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
        let mut hits: Vec<Envelope> = Vec::new();
        for item in self.scan_items(collection)? {
            let envelope = envelope_from_item(&item);
            if self.matches(&item, &envelope, filter)? {
                hits.push(envelope);
            }
        }
        eval::sort_envelopes(&mut hits, sort);
        Ok(paginate(hits, page, page_size))
    }

    /// Whether `item`, read as `envelope`, matches `filter`, its local body
    /// read when the filter holds a body clause: an item with no local body,
    /// or whose blob is gone, is matched with none.
    fn matches(
        &self,
        item: &PimdirItem,
        envelope: &Envelope,
        filter: Option<&SearchEmailsFilterQuery>,
    ) -> Result<bool> {
        let Some(filter) = filter else {
            return Ok(true);
        };
        let body = match &item.object {
            Some(hash) if reads_body(filter) => self.blobs.get(hash)?.unwrap_or_default(),
            _ => Vec::new(),
        };

        // NOTE: the summary holds the copy recipients whether or not the
        // body is local, so they are never read off the body.
        let mail = mail_summary(item.summary.as_ref());
        let copies = EnvelopeCopies {
            cc: mail
                .map(|mail| mail.cc.iter().cloned().map(address).collect())
                .unwrap_or_default(),
            bcc: mail
                .map(|mail| mail.bcc.iter().cloned().map(address).collect())
                .unwrap_or_default(),
        };

        Ok(eval::matches_filter(envelope, Some(&copies), &body, filter))
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
    /// loss, which is the cue to sync. So does one whose blob file is gone,
    /// which a purge leaves not hydrated rather than broken (pimdir GUIDE
    /// §14).
    pub fn get_message(&mut self, mailbox: &str, id: &str, seen: bool) -> Result<Vec<u8>> {
        let collection = self.hub_id(mailbox)?;
        let Some(item) = self.get(&collection, id)? else {
            bail!("Message `{id}` not found in `{mailbox}`");
        };
        let bytes = match &item.object {
            Some(hash) => self.blobs.get(hash)?,
            None => None,
        };
        let Some(bytes) = bytes else {
            return Err(CodedError::new(
                ErrorCode::BodyPending,
                format!(
                    "Message `{id}` in `{mailbox}` is not downloaded yet (body not fetched); \
                     run a sync to hydrate it"
                ),
            )
            .into());
        };

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
    ) -> Result<usize> {
        Ok(self.stage_flags(mailbox, ids, flags, op)?.len())
    }

    /// Stages a flag op on the messages of `ids` the mailbox holds,
    /// returning the queue row each message's `SetFlags` waits in, `None`
    /// for one the op leaves as it shows, for which nothing is staged.
    ///
    /// An id the mailbox does not hold is skipped, so the caller reports
    /// how many messages it found. Every found message is checked against
    /// the sources' declarations (pimdir STORAGE §15.6) before the first
    /// row is queued, so a change the check refuses on any of them leaves
    /// nothing staged. A partial support passes, its warning logged once
    /// the row is queued. Each row is still its own transaction: an
    /// enqueue that fails after the check, a declaration changing in
    /// between or the store failing, leaves the rows queued before it.
    pub fn stage_flags(
        &mut self,
        mailbox: &str,
        ids: &[&str],
        flags: &[Flag],
        op: FlagOp,
    ) -> Result<Vec<PimdirFlagged>> {
        let collection = self.hub_id(mailbox)?;
        let mut producer = self.producer()?;

        let mut checked = Vec::with_capacity(ids.len());
        for id in ids {
            let Some(item) = self.get(&collection, id)? else {
                warn!("Message `{id}` not found in `{collection}`, skipped");
                continue;
            };
            let replacement = apply_flag_op(&item.flags, flags, op)
                .with_context(|| format!("Stage flags on `{id}` in `{mailbox}`"))?;
            if replacement == item.flags {
                checked.push((*id, None));
                continue;
            }
            let action = PimdirAction::SetFlags {
                seq: item.seq,
                flags: replacement,
            };
            let partials = producer
                .check(&collection, &action)
                .map_err(pimdir_error)
                .with_context(|| format!("Stage flags on `{id}` in `{mailbox}`"))?;
            checked.push((*id, Some((action, partials))));
        }

        let mut staged = Vec::with_capacity(checked.len());
        for (id, change) in checked {
            let queue_id = match change {
                Some((action, partials)) => {
                    let queue_id = producer
                        .enqueue(&collection, &action, None)
                        .map_err(pimdir_error)
                        .with_context(|| format!("Stage flags on `{id}` in `{mailbox}`"))?;
                    for partial in &partials {
                        warn!("{partial}");
                    }
                    Some(queue_id)
                }
                None => None,
            };
            staged.push(PimdirFlagged {
                id: id.to_owned(),
                queue_id,
            });
        }
        Ok(staged)
    }

    /// The flags a message carries as the store shows it, staged changes
    /// folded in, or `None` when the store never read them.
    pub fn read_flags(&self, mailbox: &str, id: &str) -> Result<Option<Vec<String>>> {
        let collection = self.hub_id(mailbox)?;
        let Some(item) = self.get(&collection, id)? else {
            bail!("Message `{id}` not found in `{collection}`");
        };
        Ok(item.flags.known().map(|set| set.iter().cloned().collect()))
    }

    /// Stages a locally-authored message for the next sync to upload,
    /// returning its queue row and the link id it is stored under.
    ///
    /// The body is written and the row appended in one step under the
    /// producer's staging lock, the row pinning the body, so no collector
    /// runs between the two and a refused add leaves no body behind.
    ///
    /// The link id is the bare `Message-ID`, derived the way the owner
    /// derives it from the same body. One the mailbox already holds, or has
    /// a creation queued under, is refused with `item-duplicate` and nothing
    /// is staged: the owner would park the row rather than deduplicate or
    /// mint a key of its own.
    pub fn add_message(
        &mut self,
        mailbox: &str,
        flags: &[Flag],
        raw: Vec<u8>,
    ) -> Result<PimdirStaged> {
        let collection = self.hub_id(mailbox)?;
        let link_id = mail::derive(&raw).link_id;

        let flags: PimdirFlags = flags.iter().map(Flag::raw).collect();
        let enqueued = self
            .producer()?
            .enqueue_with_body_checked(&collection, &raw, |hash| PimdirAction::Add {
                link_id: Some(link_id.clone()),
                flags,
                object: Some(hash),
                supersedes: None,
            })
            .map_err(pimdir_error)
            .with_context(|| format!("Stage add in `{mailbox}`"))?;
        for partial in &enqueued.partial {
            warn!("{partial}");
        }

        Ok(PimdirStaged {
            queue_id: enqueued.id,
            message_id: link_id.0,
        })
    }

    /// Queues a message for the store's owner to send.
    ///
    /// The body is stored as given, `Bcc:` included, and the row carries the
    /// envelope derived from its headers, the `submit` intent any owner of
    /// the store may perform, naming the source that performs it. The owner
    /// holds the credentials and sends on its next run; nothing leaves from
    /// here.
    ///
    /// The row is filed under `mailbox`, which must exist: an enqueue
    /// creates the collection it names, and a send anchored on a made-up
    /// name would add a mailbox to the store.
    ///
    /// With `copy`, the intent asks the owner to file a copy in `mailbox`
    /// once the message is sent (pimdir STORAGE Annex B.2), so a failed
    /// send leaves no copy behind. Against a source that declares nothing
    /// the copy is left to the caller, an `add` beside the `submit`, as
    /// Annex B.2 asks of a producer.
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

        let mut producer = self.producer()?;
        let (source, declared) =
            match self.performer(&producer, &collection, capability::MAIL_SUBMIT)? {
                Some(source) => (source, true),
                None => (
                    self.undeclared_performer(&producer, &collection, capability::MAIL_SUBMIT)?,
                    false,
                ),
            };
        let carried = copy && declared;

        let submit = PimdirSubmit {
            source,
            from: envelope.from,
            rcpts: envelope.rcpts,
            subject: envelope.subject,
            copy: carried.then(|| PimdirCollectionId(collection.clone())),
        };
        let action = submit
            .to_action(producer.hash(&raw))
            .map_err(|err| pimdir_error(PimdirError::Action(err)))
            .with_context(|| format!("Queue send in `{mailbox}`"))?;

        let enqueued = producer
            .enqueue_with_body_checked(&collection, &raw, |_| action)
            .map_err(pimdir_error)
            .with_context(|| format!("Queue send in `{mailbox}`"))?;
        for partial in &enqueued.partial {
            warn!("{partial}");
        }
        let id = enqueued.id;
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
            .map_err(pimdir_error)
            .with_context(|| format!("Read queue row {id}"))?;
        let collection = match &status {
            PimdirActionStatus::Pending { collection, .. }
            | PimdirActionStatus::Parked { collection, .. }
            | PimdirActionStatus::Applied { collection, .. } => collection,
            PimdirActionStatus::Unknown => return Ok(status),
        };

        let ours = self
            .store
            .list_collections_by_account(self.account.as_deref())
            .map_err(pimdir_error)
            .context("List the account's collections")?
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
    pub fn create_mailbox(
        &mut self,
        name: &str,
        parent: Option<&str>,
    ) -> Result<PimdirQueuedIntent> {
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
            .map_err(pimdir_error)
            .with_context(|| format!("Queue the creation of `{name}`"))?;
        Ok(PimdirQueuedIntent { queue_id, source })
    }

    /// Queues the renaming of `mailbox` to `name` for the store's owner,
    /// returning the queue row and the source that performs it.
    ///
    /// The intent is anchored on the mailbox itself, which the payload does
    /// not restate. The mailbox keeps its parent: `name` is one level of the
    /// hierarchy. The new name arrives with the sync that performs it.
    ///
    /// The performer is named as for a creation, and a store whose sources
    /// declare nothing is refused the same way.
    pub fn rename_mailbox(&mut self, mailbox: &str, name: &str) -> Result<PimdirQueuedIntent> {
        // NOTE: the store records no hierarchy separator, so a name holding
        // the server's is the performer's to refuse: the row parks, as it
        // does on a sibling already named so.
        let collection = self.hub_id(mailbox)?;
        let mut producer = self.producer()?;
        let Some(source) = self.performer(&producer, &collection, capability::COLLECTION_RENAME)?
        else {
            bail!(
                "The sources of this store declare no capabilities, so its sync engine \
                 cannot rename a mailbox"
            );
        };

        let queue_id = producer
            .enqueue_collection_rename(&collection, name, Some(&source))
            .map_err(pimdir_error)
            .with_context(|| format!("Queue the renaming of `{collection}` to `{name}`"))?;
        Ok(PimdirQueuedIntent { queue_id, source })
    }

    /// Queues the deletion of `mailbox` and the messages it holds for the
    /// store's owner, returning the queue row and the source that performs
    /// it.
    ///
    /// The intent is anchored on the mailbox itself and never takes another
    /// one. The deletion arrives with the sync that performs it.
    ///
    /// The performer is named as for a creation, and a store whose sources
    /// declare nothing is refused the same way.
    pub fn delete_mailbox(&mut self, mailbox: &str) -> Result<PimdirQueuedIntent> {
        // NOTE: no sync engine records a mailbox's parent in the store, so
        // a mailbox holding others is the performer's to refuse: the row
        // parks naming them, and `pimdir queue show` says so.
        let collection = self.hub_id(mailbox)?;
        let mut producer = self.producer()?;
        let Some(source) = self.performer(&producer, &collection, capability::COLLECTION_DELETE)?
        else {
            bail!(
                "The sources of this store declare no capabilities, so its sync engine \
                 cannot delete a mailbox"
            );
        };

        let queue_id = producer
            .enqueue_collection_delete(&collection, Some(&source))
            .map_err(pimdir_error)
            .with_context(|| format!("Queue the deletion of `{collection}`"))?;
        Ok(PimdirQueuedIntent { queue_id, source })
    }

    /// Deletes each id `mailbox` holds, staged as `Remove` (the next sync
    /// pushes it as the backend's own disposal), returning how many it found.
    ///
    /// Every id is checked before the first row is staged, an unknown one
    /// skipped, so the caller reports how many messages it found.
    pub fn delete_messages(&mut self, mailbox: &str, ids: &[&str]) -> Result<usize> {
        let collection = self.hub_id(mailbox)?;
        let found = self.found(&collection, ids)?;
        let mut producer = self.producer()?;

        for (id, seq) in &found {
            self.enqueue(
                &mut producer,
                &collection,
                &PimdirAction::Remove { seq: *seq },
                None,
            )
            .map_err(pimdir_error)
            .with_context(|| format!("Stage delete of `{id}` in `{mailbox}`"))?;
        }
        Ok(found.len())
    }

    /// Stages one refiling action per id `from` holds, `build` deciding
    /// whether the source copy stays, returning how many it found.
    ///
    /// Every id is checked before the first row is staged, an unknown one
    /// skipped, so the caller reports how many messages it found.
    fn refile(
        &mut self,
        from: &str,
        to: &str,
        ids: &[&str],
        build: fn(i64, PimdirCollectionId) -> PimdirAction,
    ) -> Result<usize> {
        let source = self.hub_id(from)?;
        let target = PimdirCollectionId(self.hub_id(to)?);
        let found = self.found(&source, ids)?;
        let mut producer = self.producer()?;

        for (id, seq) in &found {
            self.enqueue(&mut producer, &source, &build(*seq, target.clone()), None)
                .map_err(pimdir_error)
                .with_context(|| format!("Stage refile of `{id}` from `{from}` to `{to}`"))?;
        }
        Ok(found.len())
    }

    /// The item behind a public id, or `None` when the collection holds none.
    fn get(&self, collection: &str, id: &str) -> Result<Option<PimdirItem>> {
        self.store
            .get_item(collection, parse_id(id)?)
            .map_err(pimdir_error)
            .with_context(|| format!("Read `{id}` in `{collection}`"))
    }

    /// The public ids of `ids` the collection holds, checked here so a stale
    /// id is skipped rather than parked by the owner much later; a malformed
    /// one fails.
    fn found<'a>(&self, collection: &str, ids: &[&'a str]) -> Result<Vec<(&'a str, i64)>> {
        let mut found = Vec::with_capacity(ids.len());
        for id in ids {
            match self.get(collection, id)? {
                Some(item) => found.push((*id, item.seq)),
                None => warn!("Message `{id}` not found in `{collection}`, skipped"),
            }
        }
        Ok(found)
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
                .map_err(pimdir_error)
                .with_context(|| format!("List items in `{collection}`"));
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
            .list_mail_page_filtered(&[collection], filter, None, cursor.as_ref(), limit)
            .map_err(pimdir_error)
            .with_context(|| format!("List unread items in `{collection}`"))?;
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
                .map_err(pimdir_error)
                .with_context(|| format!("List items in `{collection}`"))?;
            let n = page.len();
            all.extend(page);
            if n < SCAN_BATCH {
                break;
            }
        }
        Ok(all)
    }

    /// The mailbox's parked rows, any action kind, in append order: what the
    /// owner refused to apply and leaves to an operator (pimdir STORAGE
    /// §15.3), which no listing shows otherwise.
    pub fn parked_actions(&self, mailbox: &str) -> Result<Vec<PimdirParkedAction>> {
        let collection = self.hub_id(mailbox)?;
        let parked = self
            .store
            .parked_actions()
            .map_err(pimdir_error)
            .with_context(|| format!("List parked actions in `{mailbox}`"))?;

        Ok(parked
            .into_iter()
            .filter(|parked| parked.collection == collection)
            .collect())
    }

    /// The mailbox's pending creates and sends, in append order: the queued
    /// rows that are mail with no public id yet.
    fn queued_mail(&self, mailbox: &str) -> Result<Vec<PimdirPendingAction>> {
        let collection = self.hub_id(mailbox)?;
        let pending = self
            .store
            .pending_actions(&collection)
            .map_err(pimdir_error)
            .with_context(|| format!("List queued messages in `{mailbox}`"))?;

        Ok(pending
            .into_iter()
            .filter(|queued| is_queued_mail(&queued.action))
            .collect())
    }
}

/// Whether a queued action is a message waiting for a public id: a create,
/// or a send the owner performs.
fn is_queued_mail(action: &PimdirAction) -> bool {
    match action {
        PimdirAction::Add { .. } => true,
        PimdirAction::Unknown { kind, .. } => kind == intent::SUBMIT,
        _ => false,
    }
}

/// Builds a shared [`Envelope`] from a stored item (no body read): the
/// flags from the item, the display fields from its mail summary.
pub(crate) fn envelope_from_item(item: &PimdirItem) -> Envelope {
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
/// public `seq`.
pub(crate) fn envelope(
    id: String,
    flags: &PimdirFlags,
    summary: Option<&PimdirMailSummary>,
) -> Envelope {
    let mail = summary.cloned().unwrap_or_default();

    Envelope {
        id,
        message_id: mail.message_id,
        in_reply_to: mail.in_reply_to,
        flags: known_flags(flags),
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

/// The flags of a set as shared ones, none when the set is unknown.
fn known_flags(flags: &PimdirFlags) -> BTreeSet<Flag> {
    flags
        .known()
        .map(|flags| {
            flags
                .iter()
                .map(|raw| Flag::from_raw(raw.clone()))
                .collect()
        })
        .unwrap_or_default()
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

/// A mailbox creation, renaming or deletion queued for the owner.
#[derive(Clone, Debug)]
pub struct PimdirQueuedIntent {
    /// The `collection-create`, `collection-rename` or `collection-delete`
    /// row.
    pub queue_id: i64,
    /// The source that performs it on its server.
    pub source: String,
}

/// A flag change staged on one message for the owner.
#[derive(Clone, Debug)]
pub struct PimdirFlagged {
    /// The message's public id.
    pub id: String,
    /// The `set-flags` row it waits in, `None` when the change left the
    /// message as it shows and nothing was staged.
    pub queue_id: Option<i64>,
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
    /// The message the action carries, derived from the body it pins.
    pub envelope: PimdirQueuedEnvelope,
}

/// The message a queued creation or send carries: the shared envelope's
/// fields, its id aside.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirQueuedEnvelope {
    /// Always `None`: a create has no id until the owner applies it, a send
    /// none at all. The queue row id names the action, not the message, so
    /// it does not stand in.
    pub id: Option<String>,
    /// The bare `Message-ID:`.
    pub message_id: Option<String>,
    /// The bare ids of its `In-Reply-To:`.
    pub in_reply_to: Vec<String>,
    /// The flags it is staged with.
    pub flags: BTreeSet<Flag>,
    /// The `Subject:` header.
    pub subject: String,
    /// The senders.
    pub from: Vec<Address>,
    /// The primary recipients.
    pub to: Vec<Address>,
    /// The `Date:` header, `None` when missing or unparseable.
    pub date: Option<DateTime<FixedOffset>>,
    /// The size of the raw message, in bytes.
    pub size: u64,
    /// Whether the message carries an attachment.
    pub has_attachment: Option<bool>,
}

/// Builds a queued row from a pending add or send and the body it pins,
/// skipping every other action.
///
/// The envelope has no id: the message has none yet, and the queue row id
/// belongs to another space than the field every command reads back.
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
    let mail = mail_summary(summary.as_ref()).cloned().unwrap_or_default();

    let envelope = PimdirQueuedEnvelope {
        id: None,
        message_id: mail.message_id,
        in_reply_to: mail.in_reply_to,
        flags: known_flags(flags),
        subject: mail.subject,
        from: mail.from.into_iter().map(address).collect(),
        to: mail.to.into_iter().map(address).collect(),
        date: mail
            .date
            .as_deref()
            .and_then(|date| DateTime::parse_from_rfc3339(date).ok()),
        size: mail.size.unwrap_or(0),
        has_attachment: mail.attachment,
    };

    Some(PimdirQueued {
        id: queued.id,
        created_at: queued.created_at.clone(),
        producer: queued.producer.clone(),
        send,
        envelope,
    })
}

/// Applies a flag op to a base set, producing the replacement set `SetFlags`
/// stores. An add or a remove on an unknown base is refused: the base holds
/// no markers to build on, and the replacement would erase whatever the
/// server holds. A set replaces the whole set, so it needs no base.
///
/// Flags are matched as [`Flag`]s rather than as strings, so a marker the
/// store holds in another spelling than the one given, `\seen` for
/// `\Seen` or Thunderbird's `Junk` for `$Junk`, is the one an add keeps
/// and a remove drops.
fn apply_flag_op(current: &PimdirFlags, flags: &[Flag], op: FlagOp) -> Result<PimdirFlags> {
    if matches!(op, FlagOp::Set) {
        return apply_flag_op(&PimdirFlags::Known(Default::default()), flags, FlagOp::Add);
    }
    let Some(current) = current.known() else {
        bail!("its flags were never synced: sync first, or replace them with `flag set`");
    };
    match op {
        FlagOp::Set => unreachable!("handled above"),
        FlagOp::Add => {
            let mut set = current.clone();
            for flag in flags {
                if !set.iter().any(|raw| Flag::from_raw(raw.as_str()) == *flag) {
                    set.insert(flag.raw().to_string());
                }
            }
            Ok(PimdirFlags::Known(set))
        }
        FlagOp::Remove => {
            let mut set = current.clone();
            set.retain(|raw| !flags.contains(&Flag::from_raw(raw.as_str())));
            Ok(PimdirFlags::Known(set))
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
            Q::BeforeDate(day) => self.lower(day_key(*day)),
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

/// Whether a filter holds a body clause anywhere, which only a body read
/// answers.
fn reads_body(filter: &SearchEmailsFilterQuery) -> bool {
    use SearchEmailsFilterQuery as Q;

    match filter {
        Q::And(left, right) | Q::Or(left, right) => reads_body(left) || reads_body(right),
        Q::Not(inner) => reads_body(inner),
        Q::Body(_) => true,
        _ => false,
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

    use io_pimdir::{
        object::PimdirHash,
        placement::{PimdirLevel, PimdirLinkId},
    };

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

    /// A role name addresses the mailbox the store records it on, after the
    /// collection ids; a name does not.
    #[test]
    fn a_role_name_addresses_its_mailbox() {
        let dir = tempfile::tempdir().unwrap();
        {
            let store = io_pimdir::client::PimdirStore::open(dir.path()).unwrap();
            for id in ["imap/INBOX", "imap/Trash", "imap/Bin", "imap/Junk"] {
                store.ensure_collection(id, MAIL_KIND).unwrap();
            }
            store
                .set_collection_role("imap/INBOX", Some("inbox"))
                .unwrap();
            for id in ["imap/Trash", "imap/Bin"] {
                store.set_collection_role(id, Some("trash")).unwrap();
            }
        }
        let mut client = PimdirClient::new(PimdirConfig {
            root: dir.path().to_path_buf(),
            account: None,
        })
        .unwrap();

        assert_eq!(client.hub_id("inbox").unwrap(), "imap/INBOX");
        assert_eq!(client.hub_id("INBOX").unwrap(), "imap/INBOX");
        assert!(client.list_envelopes("inbox", None, None, false).is_ok());
        assert_eq!(client.hub_id("imap/Junk").unwrap(), "imap/Junk");
        let err = client.hub_id("junk").unwrap_err().to_string();
        assert!(err.contains("not found"), "{err}");
        // NOTE: the store keeps a role on one mailbox of an account, the
        // last one it was recorded on.
        assert_eq!(client.hub_id("trash").unwrap(), "imap/Bin");
        assert_eq!(
            client.role_mailbox_id(&MailboxRole::Inbox).unwrap(),
            Some("imap/INBOX".into())
        );
        assert_eq!(client.role_mailbox_id(&MailboxRole::Sent).unwrap(), None);
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
                        band: false,
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
                    band: false,
                }])
                .unwrap();
            store
                .write(vec![PimdirWriteOp::OpenRound {
                    collection: work,
                    scope: PimdirScope::since("2026-09-07T00:00:00Z"),
                    band: false,
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
                serde_json::to_value(&envelope).unwrap()["hasAttachment"],
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
        let added = apply_flag_op(&base, &flagged, FlagOp::Add).unwrap();
        assert!(added.contains("\\Seen") && added.contains("\\Flagged"));
        let set = apply_flag_op(&base, &flagged, FlagOp::Set).unwrap();
        assert!(!set.contains("\\Seen") && set.contains("\\Flagged"));
        let seen = [Flag::from_raw("\\Seen")];
        let removed = apply_flag_op(&base, &seen, FlagOp::Remove).unwrap();
        assert!(!removed.contains("\\Seen"));
    }

    /// An add or a remove onto an unknown set is refused: the action
    /// replaces the set, so it would erase whatever the server holds. A set
    /// replaces it anyway, so it stages a known one.
    #[test]
    fn a_flag_op_on_an_unknown_set_is_refused_but_a_set() {
        let seen = [Flag::from_raw("\\Seen")];
        for op in [FlagOp::Add, FlagOp::Remove] {
            let err = apply_flag_op(&PimdirFlags::Unknown, &seen, op)
                .unwrap_err()
                .to_string();
            assert!(err.contains("never synced"), "{err}");
        }

        let staged = apply_flag_op(&PimdirFlags::Unknown, &seen, FlagOp::Set).unwrap();
        assert_eq!(staged.known().map(BTreeSet::len), Some(1));
        assert!(staged.contains("\\Seen"));
    }

    /// A queued add carries no summary, so the row is derived from the body
    /// the action pins, the way the owner will derive it.
    #[test]
    fn a_queued_creation_renders_as_mail_with_a_null_id() {
        let queued = PimdirPendingAction {
            id: 7,
            created_at: "2026-08-27T10:00:00Z".into(),
            producer: "himalaya".into(),
            collection: "INBOX".into(),
            action: PimdirAction::Add {
                link_id: Some(PimdirLinkId("draft@x.org".into())),
                flags: known(&["\\Draft"]),
                object: Some(PimdirHash("cafe".into())),
                supersedes: None,
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
        // belongs to another space than the field commands read back, and
        // no placeholder stands in for the id the message lacks.
        let json = serde_json::to_value(&queued.envelope).unwrap();
        assert_eq!(json["id"], serde_json::Value::Null);
        assert_eq!(json["subject"], "Re: lunch");
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
        let (dir, mut client) = sent_store();
        syncs(dir.path());

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
        assert_eq!(kind, intent::SUBMIT);

        // NOTE: the `submit` payload a store owner decodes, naming the one
        // source syncing the mailbox; the message is the row's body.
        let hash = object_hash.as_ref().unwrap();
        let json: serde_json::Value = serde_json::from_str(payload).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "v": 1,
                "source": "imap",
                "from": "a@x.org",
                "rcpts": ["b@y.org", "c@y.org"],
                "subject": "hi",
            })
        );
        assert_eq!(
            PimdirSubmit::from_payload(payload).unwrap(),
            PimdirSubmit {
                source: "imap".into(),
                from: "a@x.org".into(),
                rcpts: vec!["b@y.org".into(), "c@y.org".into()],
                subject: Some("hi".into()),
                copy: None,
            }
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
        let (dir, mut client) = sent_store();
        syncs(dir.path());

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
        let PimdirAction::Unknown {
            payload,
            object_hash,
            ..
        } = &pending[0].action
        else {
            panic!("expected a submit intent, got {:?}", pending[0].action);
        };
        assert_eq!(object_hash.as_ref(), Some(&client.blobs.hash(RAW)));
        let json: serde_json::Value = serde_json::from_str(payload).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "v": 1,
                "source": "imap",
                "from": "a@x.org",
                "rcpts": ["b@y.org", "c@y.org"],
                "subject": "hi",
                "copy": "imap/Sent",
            })
        );
        assert_eq!(
            PimdirSubmit::from_payload(payload).unwrap(),
            PimdirSubmit {
                source: "imap".into(),
                from: "a@x.org".into(),
                rcpts: vec!["b@y.org".into(), "c@y.org".into()],
                subject: Some("hi".into()),
                copy: Some(PimdirCollectionId("imap/Sent".into())),
            }
        );
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

    /// A `submit` names its performer, and an undeclared one is the source
    /// syncing the mailbox: a mailbox no source syncs yet has none, and the
    /// send is refused rather than queued for nobody.
    #[test]
    fn a_send_no_source_performs_stages_nothing() {
        let (_dir, mut client) = sent_store();

        let err = client
            .send_message(Some("imap/Sent"), RAW.to_vec(), false)
            .unwrap_err();

        assert_eq!(
            pimalaya_cli::error::code_of::<ErrorCode>(&err),
            Some(ErrorCode::NoPerformer),
            "{err:#}"
        );
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
        let (dir, mut client) = sent_store();
        syncs(dir.path());
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
    fn an_add_already_queued_is_refused_and_stages_nothing() {
        let (_dir, mut client) = sent_store();
        let staged = client
            .add_message("imap/Sent", &[], DRAFT.to_vec())
            .unwrap();

        let err = client
            .add_message("imap/Sent", &[], DRAFT.to_vec())
            .unwrap_err();

        assert_eq!(
            pimalaya_cli::error::code_of::<ErrorCode>(&err),
            Some(ErrorCode::ItemDuplicate),
            "{err:#}"
        );
        let pending = client.store.pending_actions("imap/Sent").unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, staged.queue_id);
    }

    #[test]
    fn an_add_the_mailbox_holds_is_refused_and_stages_nothing() {
        let (dir, mut client) = sent_store();
        let held = PimdirMailSummary {
            message_id: Some("d1@x.org".into()),
            ..Default::default()
        };
        seed(
            dir.path(),
            "imap/Sent",
            &[Seeded {
                handle: "1".into(),
                flags: known(&[]),
                summary: Some(held),
            }],
        );

        let err = client
            .add_message("imap/Sent", &[], DRAFT.to_vec())
            .unwrap_err();

        assert_eq!(
            pimalaya_cli::error::code_of::<ErrorCode>(&err),
            Some(ErrorCode::ItemDuplicate),
            "{err:#}"
        );
        assert!(
            client
                .store
                .pending_actions("imap/Sent")
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn a_refused_add_leaves_no_body_behind() {
        let (dir, mut client) = sent_store();
        let held = PimdirMailSummary {
            message_id: Some("d1@x.org".into()),
            ..Default::default()
        };
        seed(
            dir.path(),
            "imap/Sent",
            &[Seeded {
                handle: "1".into(),
                flags: known(&[]),
                summary: Some(held),
            }],
        );

        client
            .add_message("imap/Sent", &[], DRAFT.to_vec())
            .unwrap_err();

        let body = client.blobs.path(&client.blobs.hash(DRAFT)).unwrap();
        assert!(!body.exists(), "a refused add left its body behind");
    }

    #[test]
    fn a_refused_send_leaves_no_body_behind() {
        let (dir, mut client) = sent_store();
        declare(dir.path(), &[capability::MAIL_SUBMIT]);

        let err = client
            .send_message(Some("imap/Sent"), RAW.to_vec(), true)
            .unwrap_err();

        assert_eq!(
            pimalaya_cli::error::code_of::<ErrorCode>(&err),
            Some(ErrorCode::Unsupported),
            "{err:#}"
        );
        let body = client.blobs.path(&client.blobs.hash(RAW)).unwrap();
        assert!(!body.exists(), "a refused send left its body behind");
    }

    #[test]
    fn a_sent_message_names_its_queue_row_and_message_id() {
        let (dir, mut client) = sent_store();
        syncs(dir.path());

        let sent = client
            .send_message(Some("imap/Sent"), DRAFT.to_vec(), false)
            .unwrap();

        assert_eq!(sent.staged.message_id, "d1@x.org");
        let pending = client.store.pending_actions("imap/Sent").unwrap();
        assert_eq!(pending[0].id, sent.staged.queue_id);
    }

    /// Runs `stage` on a client of a store whose staging lock is held
    /// exclusively, as the collector holds it (pimdir STORAGE §8), and
    /// checks no body reached the blob tree before the lock was released,
    /// and that the staged row landed after.
    fn assert_stages_after_the_collector(
        stage: impl FnOnce(&mut PimdirClient) -> Result<i64> + Send + 'static,
    ) {
        let (dir, mut client) = sent_store();
        syncs(dir.path());
        let body = client.blobs.path(&client.blobs.hash(DRAFT)).unwrap();

        let lock = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(dir.path().join("objects.lock"))
            .unwrap();
        lock.lock().unwrap();

        let staging = std::thread::spawn(move || {
            let id = stage(&mut client);
            (client, id)
        });
        std::thread::sleep(std::time::Duration::from_millis(500));
        assert!(
            !body.exists(),
            "a body was written while the collector held the lock"
        );

        lock.unlock().unwrap();
        let (client, id) = staging.join().unwrap();
        let id = id.unwrap();
        assert!(body.exists());
        let pending = client.store.pending_actions("imap/Sent").unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, id);
    }

    #[test]
    fn an_added_body_waits_for_the_collector() {
        assert_stages_after_the_collector(|client| {
            Ok(client
                .add_message("imap/Sent", &[], DRAFT.to_vec())?
                .queue_id)
        });
    }

    #[test]
    fn a_sent_body_waits_for_the_collector() {
        assert_stages_after_the_collector(|client| {
            Ok(client
                .send_message(Some("imap/Sent"), DRAFT.to_vec(), false)?
                .staged
                .queue_id)
        });
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

        assert_eq!(client.cancel_queued(staged.queue_id).unwrap(), "add");
        assert_eq!(
            client.queue_row(staged.queue_id).unwrap(),
            PimdirActionStatus::Unknown
        );
    }

    /// A cancel takes the owner role on the store as it stands: a store
    /// another draft wrote is refused, its stamp and its queue left as
    /// they are, never reconciled to this draft.
    #[test]
    fn a_cancel_leaves_a_store_of_another_draft_alone() {
        let (dir, mut client) = sent_store();
        let staged = client
            .add_message("imap/Sent", &[], DRAFT.to_vec())
            .unwrap();
        stamp_format(dir.path(), "storage-draft-07");

        let err = client.cancel_queued(staged.queue_id).unwrap_err();

        assert_eq!(
            pimalaya_cli::error::code_of::<ErrorCode>(&err),
            Some(ErrorCode::StoreVersion),
            "{err:#}"
        );
        assert!(
            format!("{err:#}").contains("neverest's MIGRATION.md"),
            "{err:#}"
        );
        let db = rusqlite::Connection::open(dir.path().join("pimdir.db")).unwrap();
        let (version, queued): (String, i64) = db
            .query_row(
                "SELECT format_version, (SELECT count(*) FROM queue WHERE id = ?1) \
                 FROM store_meta",
                [staged.queue_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(
            version, "storage-draft-07",
            "the cancel restamped the store"
        );
        assert_eq!(queued, 1, "the cancel dropped the row");
    }

    /// A cancel while a sync owns the store is refused at once, coded so a
    /// caller retries rather than reports a loss.
    #[test]
    fn a_cancel_during_a_sync_is_refused_as_owned() {
        let (dir, mut client) = sent_store();
        let staged = client
            .add_message("imap/Sent", &[], DRAFT.to_vec())
            .unwrap();
        // NOTE: the sync runs in another process, which holds the owner
        // lock; owners in one process share it.
        let sync = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(dir.path().join("owner.lock"))
            .unwrap();
        sync.lock().unwrap();

        let err = client.cancel_queued(staged.queue_id).unwrap_err();

        assert_eq!(
            pimalaya_cli::error::code_of::<ErrorCode>(&err),
            Some(ErrorCode::StoreOwned),
            "{err:#}"
        );
    }

    /// Stamps the store in `dir` as another draft of the format wrote it,
    /// as a store an earlier or a later neverest left.
    fn stamp_format(dir: &std::path::Path, version: &str) {
        let db = rusqlite::Connection::open(dir.join("pimdir.db")).unwrap();
        db.execute(
            "UPDATE store_meta SET format_version = ?1 WHERE id = 1",
            [version],
        )
        .unwrap();
    }

    /// A store of another draft, earlier or later, is refused pointing at
    /// the reinstall neverest's MIGRATION.md walks through, never at
    /// deleting the store.
    #[test]
    fn a_store_of_another_draft_is_refused_saying_to_reinstall_it() {
        for version in ["storage-draft-07", "storage-draft-09"] {
            let (dir, _client) = sent_store();
            stamp_format(dir.path(), version);

            let err = PimdirClient::new(PimdirConfig {
                root: dir.path().to_path_buf(),
                account: None,
            })
            .err()
            .unwrap();

            assert_eq!(
                pimalaya_cli::error::code_of::<ErrorCode>(&err),
                Some(ErrorCode::StoreVersion),
                "{err:#}"
            );
            let err = format!("{err:#}");
            assert!(err.contains(version), "{err}");
            assert!(err.contains("upgrade Himalaya"), "{err}");
            assert!(err.contains("neverest's MIGRATION.md"), "{err}");
            assert!(!err.contains("delete"), "{err}");
        }
    }

    #[test]
    fn a_database_another_program_wrote_is_refused_as_foreign() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pimdir.db");
        let db = rusqlite::Connection::open(&path).unwrap();
        db.execute_batch("CREATE TABLE notes (body TEXT)").unwrap();
        drop(db);
        // NOTE: a store others can read is refused before it is read, and
        // neither tempfile nor SQLite restricts what they create.
        #[cfg(unix)]
        for (path, mode) in [(dir.path(), 0o700), (path.as_path(), 0o600)] {
            std::fs::set_permissions(path, std::os::unix::fs::PermissionsExt::from_mode(mode))
                .unwrap();
        }

        let err = PimdirClient::new(PimdirConfig {
            root: dir.path().to_path_buf(),
            account: None,
        })
        .err()
        .unwrap();

        assert_eq!(
            pimalaya_cli::error::code_of::<ErrorCode>(&err),
            Some(ErrorCode::StoreForeign),
            "{err:#}"
        );
    }

    /// A store of this draft whose schema lacks a canonical table is
    /// refused as corrupt, never reshaped nor deleted.
    #[test]
    fn a_damaged_store_is_refused_as_corrupt() {
        let (dir, _client) = sent_store();
        let db = rusqlite::Connection::open(dir.path().join("pimdir.db")).unwrap();
        db.execute_batch("DROP TABLE item_address").unwrap();
        drop(db);

        let err = PimdirClient::new(PimdirConfig {
            root: dir.path().to_path_buf(),
            account: None,
        })
        .err()
        .unwrap();

        assert_eq!(
            pimalaya_cli::error::code_of::<ErrorCode>(&err),
            Some(ErrorCode::StoreCorrupt),
            "{err:#}"
        );
        assert!(!format!("{err:#}").contains("delete"), "{err:#}");
    }

    /// A store another user can read is refused before anything in it is
    /// read, saying to restrict it.
    #[cfg(unix)]
    #[test]
    fn a_store_others_can_read_is_refused_as_exposed() {
        use std::os::unix::fs::PermissionsExt;

        let (dir, _client) = sent_store();
        std::fs::set_permissions(dir.path(), PermissionsExt::from_mode(0o755)).unwrap();

        let err = PimdirClient::new(PimdirConfig {
            root: dir.path().to_path_buf(),
            account: None,
        })
        .err()
        .unwrap();

        assert_eq!(
            pimalaya_cli::error::code_of::<ErrorCode>(&err),
            Some(ErrorCode::StoreExposed),
            "{err:#}"
        );
        assert!(format!("{err:#}").contains("chmod 700"), "{err:#}");
    }

    /// A store two accounts share, `work` and `home`, each holding one
    /// mailbox, and a client reading each.
    fn shared_store() -> (tempfile::TempDir, PimdirClient, PimdirClient) {
        let dir = tempfile::tempdir().unwrap();
        for account in ["work", "home"] {
            io_pimdir::client::PimdirStore::open(dir.path())
                .unwrap()
                .for_account(account)
                .ensure_collection(format!("{account}/INBOX"), MAIL_KIND)
                .unwrap();
        }

        let client = |account: &str| {
            PimdirClient::new(PimdirConfig {
                root: dir.path().to_path_buf(),
                account: Some(account.into()),
            })
            .unwrap()
        };
        let (work, home) = (client("work"), client("home"));
        (dir, work, home)
    }

    #[test]
    fn a_cancel_refuses_another_accounts_row() {
        let (_dir, mut work, mut home) = shared_store();
        let staged = home.add_message("home/INBOX", &[], DRAFT.to_vec()).unwrap();

        let err = work.cancel_queued(staged.queue_id).unwrap_err();

        assert!(err.to_string().contains("account"), "{err}");
        assert_eq!(home.store.pending_actions("home/INBOX").unwrap().len(), 1);
        let ours = work.add_message("work/INBOX", &[], DRAFT.to_vec()).unwrap();
        assert_eq!(work.cancel_queued(ours.queue_id).unwrap(), "add");
    }

    #[test]
    fn a_store_two_accounts_share_is_refused_without_an_account() {
        let (dir, _work, _home) = shared_store();

        let err = PimdirClient::new(PimdirConfig {
            root: dir.path().to_path_buf(),
            account: None,
        })
        .err()
        .unwrap()
        .to_string();

        assert!(err.contains("home, work"), "{err}");
        assert!(err.contains("pimdir.account"), "{err}");
    }

    #[test]
    fn a_sent_message_reads_applied_once_its_sender_acknowledges_it() {
        let (dir, mut client) = sent_store();
        syncs(dir.path());
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

    /// Makes the source `imap` sync `imap/Sent`, declaring nothing, as an
    /// owner predating capabilities leaves it.
    fn syncs(dir: &std::path::Path) {
        io_pimdir::client::PimdirStore::open(dir)
            .unwrap()
            .for_source("imap")
            .write(vec![io_pimdir::change::PimdirWriteOp::SetCheckpoint {
                collection: io_pimdir::collection::PimdirCollectionId("imap/Sent".into()),
                checkpoint: io_pimdir::collection::PimdirCheckpoint(Vec::new()),
            }])
            .unwrap();
    }

    /// Declares `names` for the `imap` source syncing `imap/Sent`.
    fn declare(dir: &std::path::Path, names: &[&str]) {
        syncs(dir);
        let mut store = io_pimdir::client::PimdirStore::open(dir)
            .unwrap()
            .for_source("imap");
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

    /// The one pending intent of `imap/Sent`: its row id, kind and payload.
    fn intent(client: &PimdirClient) -> (i64, String, serde_json::Value) {
        let pending = client.store.pending_actions("imap/Sent").unwrap();
        assert_eq!(pending.len(), 1);
        let PimdirAction::Unknown { kind, payload, .. } = &pending[0].action else {
            panic!("expected an intent, got {:?}", pending[0].action);
        };
        let payload = serde_json::from_str(payload).unwrap();
        (pending[0].id, kind.clone(), payload)
    }

    #[test]
    fn a_mailbox_renaming_is_one_intent_on_the_mailbox() {
        let (dir, mut client) = sent_store();
        declare(dir.path(), &[capability::COLLECTION_RENAME]);

        let renamed = client.rename_mailbox("imap/Sent", "Envoyés").unwrap();

        assert_eq!(renamed.source, "imap");
        let (id, kind, payload) = intent(&client);
        assert_eq!(id, renamed.queue_id);
        assert_eq!(kind, "collection-rename");
        assert_eq!(
            payload,
            serde_json::json!({ "v": 1, "source": "imap", "name": "Envoyés" })
        );
    }

    #[test]
    fn a_mailbox_deletion_is_one_intent_on_the_mailbox() {
        let (dir, mut client) = sent_store();
        declare(dir.path(), &[capability::COLLECTION_DELETE]);

        let deleted = client.delete_mailbox("imap/Sent").unwrap();

        assert_eq!(deleted.source, "imap");
        let (id, kind, payload) = intent(&client);
        assert_eq!(id, deleted.queue_id);
        assert_eq!(kind, "collection-delete");
        assert_eq!(payload, serde_json::json!({ "v": 1, "source": "imap" }));
    }

    /// What the sync engine does once the server renamed the mailbox, its
    /// id deriving from its name (pimdir STORAGE Annex B.2): the receipt
    /// follows the collection to its new id.
    #[test]
    fn a_performed_renaming_reads_applied_under_the_new_id() {
        let (dir, mut client) = sent_store();
        declare(dir.path(), &[capability::COLLECTION_RENAME]);
        let renamed = client.rename_mailbox("imap/Sent", "Envoyés").unwrap();

        let mut owner = io_pimdir::client::PimdirStore::open(dir.path()).unwrap();
        assert!(owner.acknowledge_action(renamed.queue_id, None).unwrap());
        owner
            .rename_collection("imap/Sent", "imap/Envoyés")
            .unwrap();
        drop(owner);

        let PimdirActionStatus::Applied {
            collection, seq, ..
        } = client.queue_row(renamed.queue_id).unwrap()
        else {
            panic!("expected a performed renaming to read applied");
        };
        assert_eq!(collection, "imap/Envoyés");
        assert_eq!(seq, None);
    }

    /// What the sync engine does once the server deleted the mailbox: the
    /// receipt leaves the store with the collection, so the row reads as
    /// one never known.
    #[test]
    fn a_performed_deletion_reads_unknown_once_the_mailbox_left() {
        let (dir, mut client) = sent_store();
        declare(dir.path(), &[capability::COLLECTION_DELETE]);
        let deleted = client.delete_mailbox("imap/Sent").unwrap();

        let mut owner = io_pimdir::client::PimdirStore::open(dir.path()).unwrap();
        assert!(owner.acknowledge_action(deleted.queue_id, None).unwrap());
        assert!(matches!(
            client.queue_row(deleted.queue_id).unwrap(),
            PimdirActionStatus::Applied { .. }
        ));
        assert!(owner.delete_collection("imap/Sent").unwrap());
        drop(owner);

        assert!(matches!(
            client.queue_row(deleted.queue_id).unwrap(),
            PimdirActionStatus::Unknown
        ));
    }

    #[test]
    fn a_mailbox_renaming_or_deletion_no_source_performs_stages_nothing() {
        let (dir, mut client) = sent_store();

        // NOTE: an undeclared owner predates the intents.
        assert!(client.rename_mailbox("imap/Sent", "Envoyés").is_err());
        assert!(client.delete_mailbox("imap/Sent").is_err());

        // NOTE: each capability gates its own intent alone.
        declare(dir.path(), &[capability::COLLECTION_CREATE]);
        assert!(client.rename_mailbox("imap/Sent", "Envoyés").is_err());
        assert!(client.delete_mailbox("imap/Sent").is_err());

        declare(dir.path(), &[capability::COLLECTION_RENAME]);
        assert!(client.rename_mailbox("imap/Sent", "").is_err());
        assert!(client.rename_mailbox("imap/Sent", "a\tb").is_err());
        assert!(client.rename_mailbox("imap/Nowhere", "Envoyés").is_err());
        assert!(client.delete_mailbox("imap/Nowhere").is_err());
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

        // An add or a remove is refused on a set the store never read, and
        // the seeded mails carry some, so the flag ops pick synced ones.
        let synced = |client: &PimdirClient, ids: Vec<String>| -> Vec<String> {
            ids.into_iter()
                .filter(|id| client.read_flags("imap/INBOX", id).unwrap().is_some())
                .collect()
        };

        let seen = [Flag::from_iana(IanaFlag::Seen)];
        let read = synced(&client, ids(&inbox, 4, 1));
        client
            .store_flags("imap/INBOX", &refs(&read), &seen, FlagOp::Add)
            .unwrap();
        let unread = synced(&client, ids(&inbox, 6, 0));
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
        // The first synced mail: an add is refused on flags never read.
        let id = client
            .list_envelopes("imap/Archive", None, None, false)
            .unwrap()
            .into_iter()
            .map(|envelope| envelope.id)
            .find(|id| client.read_flags("imap/Archive", id).unwrap().is_some())
            .unwrap();
        client
            .store_flags(
                "imap/Archive",
                &[id.as_str()],
                &[Flag::from_raw("act")],
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

    /// A body clause matches against the local body, on both search paths;
    /// a message whose body is not local matches none.
    #[test]
    fn a_body_clause_reads_the_local_body() {
        let (dir, mut client) = sent_store();
        for (id, text) in [("a", "See attached."), ("b", "Nothing here.")] {
            let raw = format!(
                "Message-ID: <{id}@x.org>\r\nFrom: a@x.org\r\nDate: Thu, 14 May 2026 \
                 10:00:00 +0000\r\nSubject: {id}\r\n\r\n{text}\r\n"
            );
            client
                .add_message("imap/Sent", &[], raw.into_bytes())
                .unwrap();
        }
        let mut owner = io_pimdir::client::PimdirStore::open(dir.path())
            .unwrap()
            .for_source("imap");
        assert_eq!(owner.drain().unwrap().applied, 2);
        drop(owner);
        seed(
            dir.path(),
            "imap/Sent",
            &[Seeded {
                handle: "c".into(),
                flags: known(&[]),
                summary: Some(PimdirMailSummary {
                    message_id: Some("c@x.org".into()),
                    subject: "attached".into(),
                    date: Some("2026-05-14T10:00:00Z".into()),
                    ..Default::default()
                }),
            }],
        );

        let subjects = |client: &mut PimdirClient, query: &str| {
            let query = query.parse::<SearchEmailsQuery>().unwrap();
            let filter = query.filter.as_ref();
            let scanned = client
                .search_scanned("imap/Sent", filter, None, None, None)
                .unwrap();
            let mut subjects: Vec<String> = client
                .search_envelopes("imap/Sent", Some(&query), None, None, false)
                .unwrap()
                .into_iter()
                .map(|envelope| envelope.subject)
                .collect();
            let mut other: Vec<String> = scanned.into_iter().map(|e| e.subject).collect();
            subjects.sort();
            other.sort();
            assert_eq!(subjects, other, "the two search paths disagree");
            subjects
        };

        assert_eq!(subjects(&mut client, "body attached"), ["a"]);
        assert_eq!(
            subjects(&mut client, "not body attached"),
            ["attached", "b"]
        );
        assert_eq!(subjects(&mut client, "subject attached"), ["attached"]);
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

        let indexed = Instant::now();
        io_pimdir::client::search::PimdirIndex::open(dir.path())
            .unwrap()
            .refresh(&Default::default())
            .unwrap();
        eprintln!("indexed 50,000 mails in {:?}", indexed.elapsed());
        let options = io_pimdir::client::search::PimdirSearchOptions::default();
        for query in [
            "is:unread",
            "date:2026-08-01..",
            "from:user1",
            "subject:release",
        ] {
            let parsed = crate::pimdir::index::parse_query(&[query.to_string()], true).unwrap();
            let searched = median(|| {
                client
                    .search_mail(parsed.clone(), &options)
                    .unwrap()
                    .hits
                    .len()
            });
            let counted = median(|| client.count_mail(parsed.clone()).unwrap().0 as usize);
            eprintln!("pimdir search `{query}`: page {searched:?}, count {counted:?}");
        }
    }

    /// A store holding `imap/INBOX` with five mails: `k0` carrying no flag,
    /// `k1` the keyword `act`, `k2` `\Seen`, `k3` flags never read, and `k4`
    /// `\seen` and Thunderbird's `Junk`, spellings other than the canonical
    /// ones. `declared` is what the `imap` source declares, `None` leaving it
    /// undeclared, which nothing gates.
    fn flag_store(
        declared: Option<&[(&str, io_pimdir::capability::PimdirSupport)]>,
    ) -> (tempfile::TempDir, PimdirClient) {
        let dir = tempfile::tempdir().unwrap();
        let sets = [
            Some(&[][..]),
            Some(&["act"][..]),
            Some(&["\\Seen"][..]),
            None,
            Some(&["\\seen", "Junk"][..]),
        ];
        let mails: Vec<Seeded> = sets
            .iter()
            .enumerate()
            .map(|(i, flags)| Seeded {
                handle: format!("k{i}"),
                flags: flags.map_or(PimdirFlags::Unknown, known),
                summary: Some(PimdirMailSummary {
                    message_id: Some(format!("k{i}@x.test")),
                    subject: format!("keyword {i}"),
                    date: Some(format!("2026-05-1{i}T10:00:00Z")),
                    ..Default::default()
                }),
            })
            .collect();
        seed(dir.path(), "imap/INBOX", &mails);

        if let Some(declared) = declared {
            let mut store = io_pimdir::client::PimdirStore::open(dir.path())
                .unwrap()
                .for_source("imap");
            let declaration: Vec<_> = declared
                .iter()
                .map(|(name, support)| io_pimdir::capability::PimdirCapability {
                    collection: None,
                    name: name.to_string(),
                    support: *support,
                    detail: Some(format!("{name} as declared")),
                })
                .collect();
            store.declare("imap", &declaration).unwrap();
        }

        let client = PimdirClient::new(PimdirConfig {
            root: dir.path().to_path_buf(),
            account: None,
        })
        .unwrap();
        (dir, client)
    }

    /// The public id of the mail `k<n>` in `imap/INBOX`.
    fn id_of(client: &mut PimdirClient, n: usize) -> String {
        let message_id = format!("k{n}@x.test");
        client
            .list_envelopes("imap/INBOX", None, None, false)
            .unwrap()
            .into_iter()
            .find(|envelope| envelope.message_id.as_deref() == Some(message_id.as_str()))
            .unwrap()
            .id
    }

    fn keyword(raw: &str) -> Flag {
        Flag::from_raw(raw)
    }

    #[test]
    fn a_keyword_is_staged_and_read_back() {
        use io_pimdir::capability::{MAIL_FLAGS_KEYWORDS, PimdirSupport};

        let (_dir, mut client) = flag_store(Some(&[(MAIL_FLAGS_KEYWORDS, PimdirSupport::Full)]));
        let id = id_of(&mut client, 0);

        let staged = client
            .stage_flags(
                "imap/INBOX",
                &[id.as_str()],
                &[keyword("todo")],
                FlagOp::Add,
            )
            .unwrap();

        assert_eq!(staged.len(), 1);
        assert_eq!(staged[0].id, id);
        let pending = client.store.pending_actions("imap/INBOX").unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(Some(pending[0].id), staged[0].queue_id);
        assert_eq!(
            client.read_flags("imap/INBOX", &id).unwrap(),
            Some(vec!["todo".to_string()])
        );
    }

    /// A flag the store cannot hold, reaching the backend past the flag
    /// commands' own check, is refused by the enqueue as `flag-invalid`,
    /// and nothing is queued.
    #[test]
    fn a_flag_the_store_cannot_hold_is_refused_and_stages_nothing() {
        let (_dir, mut client) = flag_store(None);
        let id = id_of(&mut client, 0);

        for raw in ["to do", "été"] {
            let err = client
                .stage_flags("imap/INBOX", &[id.as_str()], &[keyword(raw)], FlagOp::Add)
                .unwrap_err();

            assert_eq!(
                pimalaya_cli::error::code_of::<ErrorCode>(&err),
                Some(ErrorCode::FlagInvalid),
                "{err:#}"
            );
        }
        assert!(
            client
                .store
                .pending_actions("imap/INBOX")
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn a_keyword_is_removed_alone_and_dropped_by_a_set() {
        let (_dir, mut client) = flag_store(None);
        let act = id_of(&mut client, 1);

        client
            .stage_flags(
                "imap/INBOX",
                &[act.as_str()],
                &[keyword("todo")],
                FlagOp::Add,
            )
            .unwrap();
        client
            .stage_flags(
                "imap/INBOX",
                &[act.as_str()],
                &[keyword("act")],
                FlagOp::Remove,
            )
            .unwrap();
        assert_eq!(
            client.read_flags("imap/INBOX", &act).unwrap(),
            Some(vec!["todo".to_string()])
        );

        client
            .stage_flags(
                "imap/INBOX",
                &[act.as_str()],
                &[Flag::from_iana(IanaFlag::Seen)],
                FlagOp::Set,
            )
            .unwrap();
        assert_eq!(
            client.read_flags("imap/INBOX", &act).unwrap(),
            Some(vec!["\\Seen".to_string()])
        );
    }

    #[test]
    fn a_source_without_keywords_refuses_one_and_stages_nothing() {
        use io_pimdir::capability::{MAIL_FLAGS_KEYWORDS, MAIL_FLAGS_SEEN, PimdirSupport};

        let (_dir, mut client) = flag_store(Some(&[
            (MAIL_FLAGS_SEEN, PimdirSupport::Full),
            (MAIL_FLAGS_KEYWORDS, PimdirSupport::None),
        ]));
        let id = id_of(&mut client, 0);

        let err = client
            .stage_flags(
                "imap/INBOX",
                &[id.as_str()],
                &[keyword("todo")],
                FlagOp::Add,
            )
            .unwrap_err();
        assert_eq!(
            pimalaya_cli::error::code_of::<ErrorCode>(&err),
            Some(ErrorCode::Unsupported),
            "{err:#}"
        );
        let err = format!("{err:#}");

        assert!(
            err.contains(
                "Source imap does not support mail.flags.keywords: mail.flags.keywords as declared"
            ),
            "{err}"
        );
        assert!(
            client
                .store
                .pending_actions("imap/INBOX")
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn a_flag_held_in_another_spelling_is_matched_as_that_flag() {
        let (_dir, mut client) = flag_store(None);
        let id = id_of(&mut client, 4);

        client
            .stage_flags(
                "imap/INBOX",
                &[id.as_str()],
                &[Flag::from_iana(IanaFlag::Seen)],
                FlagOp::Add,
            )
            .unwrap();
        assert_eq!(
            client.read_flags("imap/INBOX", &id).unwrap(),
            Some(vec!["Junk".to_string(), "\\seen".to_string()])
        );

        client
            .stage_flags(
                "imap/INBOX",
                &[id.as_str()],
                &[
                    Flag::from_iana(IanaFlag::Seen),
                    Flag::from_iana(IanaFlag::Junk),
                ],
                FlagOp::Remove,
            )
            .unwrap();
        assert_eq!(client.read_flags("imap/INBOX", &id).unwrap(), Some(vec![]));
    }

    #[test]
    fn an_add_or_remove_on_flags_never_synced_is_refused_and_a_set_staged() {
        let (_dir, mut client) = flag_store(None);
        let unread = id_of(&mut client, 3);
        let other = id_of(&mut client, 0);

        for op in [FlagOp::Add, FlagOp::Remove] {
            let err = client
                .store_flags(
                    "imap/INBOX",
                    &[other.as_str(), unread.as_str()],
                    &[Flag::from_iana(IanaFlag::Flagged)],
                    op,
                )
                .unwrap_err();
            let err = format!("{err:#}");
            assert!(err.contains("never synced"), "{err}");
            assert!(err.contains(&format!("`{unread}`")), "{err}");
        }
        assert!(
            client
                .store
                .pending_actions("imap/INBOX")
                .unwrap()
                .is_empty()
        );

        client
            .stage_flags(
                "imap/INBOX",
                &[unread.as_str()],
                &[Flag::from_iana(IanaFlag::Flagged)],
                FlagOp::Set,
            )
            .unwrap();
        assert_eq!(
            client.read_flags("imap/INBOX", &unread).unwrap(),
            Some(vec!["\\Flagged".to_string()])
        );
    }

    #[test]
    fn flags_never_read_list_as_none() {
        let (_dir, mut client) = flag_store(None);
        let id = id_of(&mut client, 3);

        assert_eq!(client.read_flags("imap/INBOX", &id).unwrap(), None);
    }

    #[test]
    fn a_source_with_keywords_in_part_stages_one() {
        use io_pimdir::capability::{MAIL_FLAGS_KEYWORDS, PimdirSupport};

        let (_dir, mut client) = flag_store(Some(&[(MAIL_FLAGS_KEYWORDS, PimdirSupport::Partial)]));
        let id = id_of(&mut client, 0);

        let staged = client
            .stage_flags(
                "imap/INBOX",
                &[id.as_str()],
                &[keyword("todo")],
                FlagOp::Add,
            )
            .unwrap();

        assert_eq!(staged.len(), 1);
        assert_eq!(client.store.pending_actions("imap/INBOX").unwrap().len(), 1);
    }

    /// A source stating `mail.flags.junk` gates `$Junk` by that row, not by
    /// `mail.flags.keywords` (pimdir STORAGE Annex B.1).
    #[test]
    fn a_junk_mark_is_gated_by_its_own_row() {
        use io_pimdir::capability::{MAIL_FLAGS_JUNK, MAIL_FLAGS_KEYWORDS, PimdirSupport};

        let (_dir, mut client) = flag_store(Some(&[
            (MAIL_FLAGS_KEYWORDS, PimdirSupport::None),
            (MAIL_FLAGS_JUNK, PimdirSupport::Full),
        ]));
        let id = id_of(&mut client, 0);
        let junk = Flag::from_iana(IanaFlag::Junk);

        let staged = client
            .stage_flags("imap/INBOX", &[id.as_str()], &[junk], FlagOp::Add)
            .unwrap();
        assert_eq!(staged.len(), 1);

        let (_dir, mut client) = flag_store(Some(&[
            (MAIL_FLAGS_KEYWORDS, PimdirSupport::Full),
            (MAIL_FLAGS_JUNK, PimdirSupport::None),
        ]));
        let id = id_of(&mut client, 0);
        let junk = Flag::from_iana(IanaFlag::Junk);

        let err = client
            .stage_flags("imap/INBOX", &[id.as_str()], &[junk], FlagOp::Add)
            .unwrap_err();
        assert!(format!("{err:#}").contains(MAIL_FLAGS_JUNK), "{err:#}");
        assert!(
            client
                .store
                .pending_actions("imap/INBOX")
                .unwrap()
                .is_empty()
        );
    }

    /// The shared `flag set -f seen` on three mails, which drops `act` from
    /// `k1` only: the one refusal stages nothing for the other two either.
    #[test]
    fn a_refusal_part_way_stages_nothing() {
        use io_pimdir::capability::{MAIL_FLAGS_KEYWORDS, MAIL_FLAGS_SEEN, PimdirSupport};

        let (_dir, mut client) = flag_store(Some(&[
            (MAIL_FLAGS_SEEN, PimdirSupport::Full),
            (MAIL_FLAGS_KEYWORDS, PimdirSupport::None),
        ]));
        let ids: Vec<String> = (0..3).map(|n| id_of(&mut client, n)).collect();
        let ids: Vec<&str> = ids.iter().map(String::as_str).collect();

        let err = client
            .store_flags(
                "imap/INBOX",
                &ids,
                &[Flag::from_iana(IanaFlag::Seen)],
                FlagOp::Set,
            )
            .unwrap_err();
        let err = format!("{err:#}");

        assert!(err.contains(MAIL_FLAGS_KEYWORDS), "{err}");
        assert!(err.contains(&format!("`{}`", ids[1])), "{err}");
        assert!(
            client
                .store
                .pending_actions("imap/INBOX")
                .unwrap()
                .is_empty()
        );
    }

    /// An id the mailbox does not hold is skipped and counted out, as the
    /// shared commands report how many of the given ids they found.
    #[test]
    fn an_unknown_id_is_skipped_and_counted_out() {
        let (_dir, mut client) = flag_store(None);
        let id = id_of(&mut client, 0);
        let seen = [Flag::from_iana(IanaFlag::Seen)];

        let count = client
            .store_flags("imap/INBOX", &[id.as_str(), "999"], &seen, FlagOp::Add)
            .unwrap();
        assert_eq!(count, 1);
        assert_eq!(client.store.pending_actions("imap/INBOX").unwrap().len(), 1);

        let staged = client
            .stage_flags("imap/INBOX", &["998", "999"], &seen, FlagOp::Add)
            .unwrap();
        assert!(staged.is_empty());
        assert_eq!(client.store.pending_actions("imap/INBOX").unwrap().len(), 1);
    }

    /// A refile or a delete stages the ids the mailbox holds and counts them,
    /// an unknown one skipped; none known stages nothing.
    #[test]
    fn a_refile_or_delete_stages_the_ids_it_found() {
        let (dir, mut client) = flag_store(None);
        seed(dir.path(), "imap/Archive", &[]);
        let ids: Vec<String> = (0..3).map(|n| id_of(&mut client, n)).collect();
        let queued = |client: &PimdirClient| client.store.list_pending_actions().unwrap().len();

        let copied = client.copy_messages("imap/INBOX", "imap/Archive", &[&ids[0], "999"]);
        assert_eq!(copied.unwrap(), 1);
        assert_eq!(queued(&client), 1);
        let moved = client.move_messages("imap/INBOX", "imap/Archive", &[&ids[1], "999"]);
        assert_eq!(moved.unwrap(), 1);
        assert_eq!(queued(&client), 2);
        let deleted = client.delete_messages("imap/INBOX", &[&ids[2], "999"]);
        assert_eq!(deleted.unwrap(), 1);
        assert_eq!(queued(&client), 3);

        let none = client.copy_messages("imap/INBOX", "imap/Archive", &["998", "999"]);
        assert_eq!(none.unwrap(), 0);
        assert!(
            client
                .delete_messages("imap/INBOX", &[&ids[0], "x"])
                .is_err()
        );
        assert_eq!(queued(&client), 3);
    }

    /// A held item whose blob file is gone reads as not local yet, the
    /// cue to sync, and not as a loss (pimdir GUIDE §14).
    #[test]
    fn a_body_gone_from_the_blob_tree_reads_as_pending() {
        let (dir, mut client) = sent_store();
        client
            .add_message("imap/Sent", &[], DRAFT.to_vec())
            .unwrap();
        let mut owner = io_pimdir::client::PimdirStore::open(dir.path())
            .unwrap()
            .for_source("imap");
        assert_eq!(owner.drain().unwrap().applied, 1);
        drop(owner);
        let id = client
            .list_envelopes("imap/Sent", None, None, false)
            .unwrap()[0]
            .id
            .clone();
        let item = client.get("imap/Sent", &id).unwrap().unwrap();
        std::fs::remove_file(client.blobs.path(item.object.as_ref().unwrap()).unwrap()).unwrap();

        let err = client.get_message("imap/Sent", &id, false).unwrap_err();

        assert_eq!(
            pimalaya_cli::error::code_of::<ErrorCode>(&err),
            Some(ErrorCode::BodyPending),
            "{err}"
        );
    }

    /// A flag change leaving a message as it shows stages nothing for it,
    /// and still counts it found.
    #[test]
    fn a_flag_change_the_message_shows_already_stages_nothing() {
        let (_dir, mut client) = flag_store(None);
        let seen = [Flag::from_iana(IanaFlag::Seen)];
        let ids = [id_of(&mut client, 2), id_of(&mut client, 4)];
        let ids: Vec<&str> = ids.iter().map(String::as_str).collect();

        let count = client
            .store_flags("imap/INBOX", &ids, &seen, FlagOp::Add)
            .unwrap();

        assert_eq!(count, 2);
        assert!(
            client
                .store
                .pending_actions("imap/INBOX")
                .unwrap()
                .is_empty()
        );
    }

    /// A row the owner parked shows in the mailbox it was queued in, with
    /// why, and is cancelled as a pending one is.
    #[test]
    fn a_parked_row_is_listed_and_cancelled() {
        let (dir, mut client) = sent_store();
        let twin = client
            .add_message("imap/Sent", &[], DRAFT.to_vec())
            .unwrap();
        // NOTE: the source lists the same message before the owner drains
        // the add, which then parks.
        let listed = PimdirMailSummary {
            message_id: Some("d1@x.org".into()),
            ..Default::default()
        };
        seed(
            dir.path(),
            "imap/Sent",
            &[Seeded {
                handle: "1".into(),
                flags: known(&[]),
                summary: Some(listed),
            }],
        );
        let mut owner = io_pimdir::client::PimdirStore::open(dir.path())
            .unwrap()
            .for_source("imap");
        owner.drain().unwrap();
        drop(owner);

        let parked = client.parked_actions("imap/Sent").unwrap();
        assert_eq!(parked.len(), 1, "{parked:?}");
        assert_eq!(parked[0].id, twin.queue_id);
        assert_eq!(parked[0].action, "add");
        assert!(!parked[0].error.is_empty());

        assert_eq!(client.cancel_queued(twin.queue_id).unwrap(), "add");
        assert!(client.parked_actions("imap/Sent").unwrap().is_empty());
    }
}
