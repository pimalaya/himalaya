//! # pimdir backend
//!
//! The pimdir adapter of the unified client, reading the
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

use std::{
    collections::{BTreeMap, BTreeSet},
    ops::ControlFlow,
};

use anyhow::{Context, Result, anyhow, bail};
use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime};
use io_pimdir::{
    capability,
    client::{
        PimdirError,
        producer::{PimdirActionStatus, PimdirEnqueued, PimdirParkedAction},
        reader::{
            PimdirCollection, PimdirItem, PimdirMailCursor, PimdirMailFilter, PimdirRoundState,
        },
    },
    codec::{PimdirAction, PimdirActionHandle, action_from_payload},
    collection::{self, PimdirCollectionId, PimdirCoverage},
    intent::{self, PimdirSubmit},
    object::PimdirHash,
    placement::PimdirFlags,
    summary::{
        PimdirAddress, PimdirSummary,
        mail::{self, PimdirMailSummary},
    },
};
use log::{info, warn};
use pimalaya_cli::{capability::CapabilityLevel, error::CodedError};

use crate::{
    capability::{Capability, StoreSupport, missing},
    email::{
        address::Address,
        envelope::{Envelope, EnvelopePage, page_follows},
        flag::{Flag, FlagOp, IanaFlag},
        mailbox::{Mailbox, MailboxIndex, MailboxRole},
        outcome::Outcome,
        search::{
            eval::{self, EnvelopeCopies},
            filter::query::SearchEmailsFilterQuery,
            query::SearchEmailsQuery,
            sort::{
                eval::sort_envelopes,
                query::{SearchEmailsSorter, SearchEmailsSorterKind, SearchEmailsSorterOrder},
            },
        },
        submission::SubmissionEnvelope,
    },
    error::{
        ErrorCode, ItemError, existing_mailbox, mailbox_not_found, message_not_found,
        missing_message, pimdir_error,
    },
    pimdir::{
        client::{PimdirClient, handle},
        queue::list::{PimdirQueueRefused, PimdirQueueRow},
        status::{
            PimdirMailboxStatus, PimdirParkedStatus, PimdirRefusedStatus, PimdirStatusOutput,
        },
    },
};

/// The mail media type a pimdir collection carries to be a mailbox.
pub(crate) const MAIL_KIND: &str = "message/rfc822";

/// How many items to pull per keyset page when scanning a whole collection.
const SCAN_BATCH: usize = 500;

/// The smallest keyset page a filtered search reads, so a sparse match
/// does not cost a query per row.
const WALK_MIN: usize = 64;

/// The sort key of a dated mail: its summary date (pimdir STORAGE Annex A.1).
const KEY_FORMAT: &str = "%Y-%m-%dT%H:%M:%SZ";

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

        Err(mailbox_not_found(format!(
            "Mailbox `{mailbox}` not found in the pimdir store, which holds: {}",
            ids.join(", "),
        )))
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

    /// The ids of the account's mail collections, in store order, as
    /// `mailbox list` prints them.
    pub fn mail_collection_ids(&self) -> Result<Vec<String>> {
        let collections = self.mail_collections()?;
        Ok(collections
            .into_iter()
            .map(|collection| collection.id)
            .collect())
    }

    /// The account's mail collections, in store order.
    fn mail_collections(&self) -> Result<Vec<PimdirCollection>> {
        Ok(self
            .store
            .list_collections_by_account(self.account.as_deref())
            .map_err(pimdir_error)
            .context("List pimdir collections")?
            .into_iter()
            .filter(|held| collection::media_type(&held.kind) == MAIL_KIND)
            .collect())
    }

    /// How the store serves a unified write, read from what its sources
    /// declare (pimdir STORAGE §15.6), nothing staged: `None` for a read or
    /// an account feature, and for a store whose sources declare nothing,
    /// where the backend's static row stands.
    ///
    /// An intent (`collection.*`, `mail.submit`) is served when a source
    /// performs it. An item write runs through the store's own gate in
    /// each of the account's mail collections: refused in every one it is
    /// `none`, in some of them or passing in part it is `partial`.
    pub fn store_support(&self, capability: Capability) -> Result<Option<StoreSupport>> {
        let Some(name) = capability.annex_b_name() else {
            return Ok(None);
        };

        let collections = self.mail_collections()?;
        let mut declared = false;
        let mut refusals = Vec::new();
        let mut partials = BTreeSet::new();

        for collection in &collections {
            let sources = self
                .store
                .capabilities(&collection.id)
                .map_err(pimdir_error)
                .with_context(|| format!("Read the capabilities of `{}`", collection.id))?;
            declared |= sources.iter().any(|source| source.declared.is_some());

            match capability::check(&[name.to_string()], &sources) {
                Ok(passed) => partials.extend(passed.into_iter().map(|partial| {
                    let detail = partial.detail.unwrap_or_else(|| String::from("in part"));
                    format!("{}: {detail}", partial.source)
                })),
                Err(refusal) => refusals.push(refusal.to_string()),
            }
        }

        if !declared {
            return Ok(None);
        }

        if name.starts_with("collection.") || name.starts_with("mail.submit") {
            let (candidates, _) = self.performers(name)?;
            let support = if candidates.is_empty() {
                StoreSupport {
                    level: CapabilityLevel::None,
                    detail: Some(format!("no source of the store performs `{name}`")),
                }
            } else {
                StoreSupport {
                    level: CapabilityLevel::Full,
                    detail: None,
                }
            };
            return Ok(Some(support));
        }

        let support = match refusals.first() {
            Some(refusal) if refusals.len() == collections.len() => StoreSupport {
                level: CapabilityLevel::None,
                detail: Some(refusal.clone()),
            },
            Some(refusal) => StoreSupport {
                level: CapabilityLevel::Partial,
                detail: Some(format!(
                    "refused in {} of {} mailboxes: {refusal}",
                    refusals.len(),
                    collections.len()
                )),
            },
            None if !partials.is_empty() => StoreSupport {
                level: CapabilityLevel::Partial,
                detail: Some(partials.into_iter().collect::<Vec<_>>().join("; ")),
            },
            None => StoreSupport {
                level: CapabilityLevel::Full,
                detail: None,
            },
        };

        Ok(Some(support))
    }

    /// Lists the account's mail collections, each addressed by its collection
    /// id and named by the collection row's own name, sorted by id.
    /// `with_counts` fills `total` with the live item count and `unread`
    /// as [`Self::list_covered_mailboxes`] counts it.
    pub fn list_mailboxes(&mut self, with_counts: bool) -> Result<Vec<Mailbox>> {
        let collections = self.mail_collections()?;

        let unread = if with_counts {
            self.count_unread(&collections)?
        } else {
            Default::default()
        };

        let mut mailboxes = Vec::new();
        for collection in collections {
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
            let unread = with_counts.then(|| unread.get(&collection.id).copied().unwrap_or(0));
            mailboxes.push(Mailbox {
                id: collection.id,
                name: collection.name,
                role: collection.role.as_deref().map(MailboxRole::parse),
                total,
                unread,
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
        let unread = self.count_unread(&collections)?;

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

    /// The unread mail of each collection as the overlaying reader lists
    /// it: the store's unread count, which reads committed rows only, where
    /// no queued action changes what the mailbox shows
    /// (`PimdirReader::overlay_quiet`), else its items walked and those
    /// without `\Seen` counted, as that count reads them.
    fn count_unread(&self, collections: &[PimdirCollection]) -> Result<BTreeMap<String, u64>> {
        let ids: Vec<&str> = collections.iter().map(|c| c.id.as_str()).collect();
        let mut unread = self
            .store
            .count_unread(&ids, None, None)
            .map_err(pimdir_error)
            .context("Count unread pimdir mail")?;

        for id in ids {
            let quiet = self
                .store
                .overlay_quiet(id)
                .map_err(pimdir_error)
                .with_context(|| format!("Read the queue of `{id}`"))?;
            if quiet {
                continue;
            }

            let mut count = 0;
            self.walk(id, None, false, SCAN_BATCH, |item| {
                if !item.flags.contains("\\Seen") {
                    count += 1;
                }
                ControlFlow::Continue(SCAN_BATCH)
            })?;
            unread.insert(id.to_owned(), count);
        }

        Ok(unread)
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
        with_attachment: bool,
    ) -> Result<Vec<Envelope>> {
        self.list_envelopes_counted(mailbox, page, page_size, with_attachment)
            .map(|(page, _)| page.envelopes)
    }

    /// Lists envelopes as [`Self::list_envelopes`] does, beside how many
    /// messages the collection holds: the store's live item count, a local
    /// query rather than a walk of every item, which also says whether a
    /// page follows.
    pub fn list_envelopes_counted(
        &mut self,
        mailbox: &str,
        page: Option<u32>,
        page_size: Option<u32>,
        with_attachment: bool,
    ) -> Result<(EnvelopePage, Option<u64>)> {
        let collection = self.hub_id(mailbox)?;
        let total = self
            .store
            .count_items(&collection)
            .map_err(pimdir_error)
            .with_context(|| format!("Count items in `{collection}`"))?;
        let envelopes =
            self.list_collection_envelopes(&collection, page, page_size, with_attachment)?;
        let more = page_follows(total, page, page_size);

        Ok((EnvelopePage { envelopes, more }, Some(total)))
    }

    /// The envelopes of `collection`, as [`Self::list_envelopes`] lists
    /// them.
    fn list_collection_envelopes(
        &mut self,
        collection: &str,
        page: Option<u32>,
        page_size: Option<u32>,
        _with_attachment: bool,
    ) -> Result<Vec<Envelope>> {
        let Some(wanted) = page_end(page, page_size) else {
            let envelopes = self
                .scan_items(collection)?
                .iter()
                .map(envelope_from_item)
                .collect();
            return Ok(envelopes);
        };
        if wanted == 0 {
            return Ok(Vec::new());
        }

        let mut envelopes: Vec<Envelope> = Vec::new();
        self.walk(collection, None, false, wanted.min(SCAN_BATCH), |item| {
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
    /// stops one hit past the end of the page when the order is the store's
    /// own (`PimdirPushdown`). Every clause is still matched in memory.
    pub fn search_envelopes(
        &mut self,
        mailbox: &str,
        query: Option<&SearchEmailsQuery>,
        page: Option<u32>,
        page_size: Option<u32>,
        _with_attachment: bool,
    ) -> Result<EnvelopePage> {
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
    ) -> Result<Option<EnvelopePage>> {
        if page_size == Some(0) {
            return Ok(Some(EnvelopePage::default()));
        }

        let pushdown = filter.map(PimdirPushdown::of).unwrap_or_default();
        let unread = pushdown.unread
            && self
                .store
                .overlay_quiet(collection)
                .map_err(pimdir_error)
                .with_context(|| format!("Read the queue of `{collection}`"))?;
        // NOTE: one hit past the page says whether another follows; a page
        // in another order sits anywhere in the range, so it is read whole.
        let wanted = match is_store_order(sort) {
            true => page_end(page, page_size).map(|end| end.saturating_add(1)),
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
        sort_envelopes(&mut hits, sort);

        let more = page_follows(hits.len() as u64, page, page_size);
        let envelopes = paginate(hits, page, page_size);

        Ok(Some(EnvelopePage { envelopes, more }))
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
    ) -> Result<EnvelopePage> {
        let mut hits: Vec<Envelope> = Vec::new();
        for item in self.scan_items(collection)? {
            let envelope = envelope_from_item(&item);
            if self.matches(&item, &envelope, filter)? {
                hits.push(envelope);
            }
        }
        sort_envelopes(&mut hits, sort);

        let more = page_follows(hits.len() as u64, page, page_size);
        let envelopes = paginate(hits, page, page_size);

        Ok(EnvelopePage { envelopes, more })
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

    /// The mailbox's pending rows, every action kind, in append order, as
    /// `pimdir queue list` prints them.
    ///
    /// A queued action carries no summary, the owner deriving one from the
    /// body when it applies the action, so the subject is read off the body
    /// the action pins, derived the same way.
    ///
    /// A row whose payload does not decode leaves the mailbox's rows
    /// unread: the mailbox is then the one item error.
    pub fn queued_rows(&self, mailbox: &str) -> Result<Outcome<Vec<PimdirQueueRow>>> {
        let collection = self.hub_id(mailbox)?;
        let queued = match self.store.pending_actions(&collection) {
            Ok(queued) => queued,
            Err(err @ PimdirError::Action(_)) => {
                let err = pimdir_error(err)
                    .context(format!("Cannot decode a queued row of `{collection}`"));
                return Ok(Outcome {
                    value: Vec::new(),
                    errors: vec![ItemError::new(Some(&collection), &err)],
                });
            }
            Err(err) => {
                let context = format!("List the queued rows of `{mailbox}`");
                return Err(pimdir_error(err).context(context));
            }
        };
        let identity = self.store_identity()?;

        let rows: Vec<PimdirQueueRow> = queued
            .into_iter()
            .map(|queued| {
                let addressed = self.addressed(&queued.collection, &queued.action);

                PimdirQueueRow {
                    row: queued.id,
                    handle: handle(&identity, queued.id),
                    kind: queued.action.kind().to_owned(),
                    collection: queued.collection,
                    link_id: addressed.link_id,
                    item_id: addressed.item_id,
                    producer: queued.producer,
                    queued_at: queued.created_at,
                    attempts: u64::try_from(queued.attempts).unwrap_or_default(),
                    unconfirmed: false,
                    error: None,
                    subject: addressed.subject,
                }
            })
            .collect();

        Ok(Outcome::from(rows))
    }

    /// The refusals standing in the mailbox, per message and source, as
    /// `pimdir queue list` prints them beside its rows (pimdir STORAGE
    /// §15.4): a change applied in the store that a server refused for good,
    /// which the sync engine holds until the message changes again.
    pub fn refused_rows(&self, mailbox: &str) -> Result<Vec<PimdirQueueRefused>> {
        let collection = self.hub_id(mailbox)?;
        let refusals = self
            .store
            .refusals(&collection)
            .map_err(pimdir_error)
            .with_context(|| format!("Read the refusals of pimdir mailbox `{mailbox}`"))?;

        Ok(refusals
            .into_iter()
            .map(|refusal| PimdirQueueRefused {
                collection: collection.clone(),
                item_id: refusal.seq.to_string(),
                source: refusal.source,
                reason: refusal.reason,
                refused_at: refusal.refused_at,
            })
            .collect())
    }

    /// The mailbox's parked rows, any action kind, in append order, as
    /// `pimdir queue list` prints them.
    pub fn parked_rows(&self, mailbox: &str) -> Result<Vec<PimdirQueueRow>> {
        let parked = self.parked_actions(mailbox)?;
        let identity = self.store_identity()?;

        Ok(parked
            .into_iter()
            .map(|parked| {
                // NOTE: a payload the store parked for not decoding
                // addresses nothing Himalaya can name.
                let addressed = action_from_payload(&parked.action, &parked.payload)
                    .map(|action| self.addressed(&parked.collection, &action))
                    .unwrap_or_default();

                PimdirQueueRow {
                    row: parked.id,
                    handle: handle(&identity, parked.id),
                    unconfirmed: parked.unconfirmed().is_some(),
                    kind: parked.action,
                    collection: parked.collection,
                    link_id: addressed.link_id,
                    item_id: addressed.item_id,
                    producer: parked.producer,
                    queued_at: parked.created_at,
                    attempts: u64::try_from(parked.attempts).unwrap_or_default(),
                    error: Some(parked.error),
                    subject: addressed.subject,
                }
            })
            .collect())
    }

    /// What a row of `collection` holding `action` addresses: the message
    /// it creates by its link id, the stored message it acts on by its
    /// public id, and the subject of the body it pins, else of the message
    /// it acts on. A parked `submit` names no body, its payload's subject
    /// standing in.
    fn addressed(&self, collection: &str, action: &PimdirAction) -> Addressed {
        let pinned = |hash: &PimdirHash| {
            let body = self.blobs.get(hash).ok().flatten()?;
            let summary = mail::derive(&body).summary;
            mail_summary(summary.as_ref()).map(|mail| mail.subject.clone())
        };

        let stored = |seq: i64| {
            let item = self.store.get_item(collection, seq).ok().flatten();
            let subject = item.and_then(|item| {
                mail_summary(item.summary.as_ref()).map(|mail| mail.subject.clone())
            });

            Addressed {
                item_id: Some(seq.to_string()),
                subject,
                ..Default::default()
            }
        };

        match action {
            PimdirAction::Add {
                link_id, object, ..
            } => Addressed {
                link_id: link_id.as_ref().map(|id| id.0.clone()),
                subject: object.as_ref().and_then(pinned),
                ..Default::default()
            },
            PimdirAction::Update { seq, object, .. } => {
                let addressed = stored(*seq);

                Addressed {
                    subject: pinned(object).or(addressed.subject),
                    ..addressed
                }
            }
            PimdirAction::SetFlags { seq, .. }
            | PimdirAction::Remove { seq, .. }
            | PimdirAction::Move { seq, .. }
            | PimdirAction::Copy { seq, .. } => stored(*seq),
            PimdirAction::Unknown {
                kind,
                payload,
                object_hash,
            } if kind == intent::SUBMIT => {
                let declared = || PimdirSubmit::from_payload(payload).ok()?.subject;

                Addressed {
                    subject: object_hash.as_ref().and_then(pinned).or_else(declared),
                    ..Default::default()
                }
            }
            PimdirAction::SetPerformer { .. } | PimdirAction::Unknown { .. } => {
                Addressed::default()
            }
        }
    }

    /// Reads one message's raw bytes from its content-addressed blob, and
    /// its flags.
    ///
    /// An item with no local body fails as not fetched rather than as a data
    /// loss, which is the cue to sync. So does one whose blob file is gone,
    /// which a purge leaves not hydrated rather than broken (pimdir GUIDE
    /// §14).
    pub fn get_message(
        &mut self,
        mailbox: &str,
        id: &str,
        seen: bool,
    ) -> Result<(Vec<u8>, BTreeSet<Flag>)> {
        let collection = self.hub_id(mailbox)?;
        let Some(item) = self.get(&collection, id)? else {
            return Err(message_not_found(format!(
                "Message `{id}` not found in `{mailbox}`"
            )));
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
        let mut flags = known_flags(&item.flags);

        if seen {
            let seen_flag = Flag::from_iana(IanaFlag::Seen);
            flags.insert(seen_flag.clone());
            self.store_flags(mailbox, &[id], &[seen_flag], FlagOp::Add)
                .with_context(|| format!("Cannot stage \\Seen on `{id}` in `{mailbox}`"))?;
        }

        Ok((bytes, flags))
    }

    /// Adds, sets, or removes `flags` on an id set, staged as `SetFlags`.
    ///
    /// The action carries the whole set the message is left with. An add
    /// or a remove also states the set it was read from (pimdir STORAGE
    /// §15.3), so the owner applies the change alone to the set the
    /// message holds by then, and a flag a sync landed or removed while
    /// the row waited survives. A set states none: it replaces whatever
    /// the message holds.
    pub fn store_flags(
        &mut self,
        mailbox: &str,
        ids: &[&str],
        flags: &[Flag],
        op: FlagOp,
    ) -> Result<Outcome<usize>> {
        Ok(self
            .stage_flag_op(mailbox, ids, flags, op)?
            .map(|staged| staged.len()))
    }

    /// Stages a flag op as `Self::stage_flag_op` does, an id the mailbox
    /// does not hold skipped with a warning.
    pub fn stage_flags(
        &mut self,
        mailbox: &str,
        ids: &[&str],
        flags: &[Flag],
        op: FlagOp,
    ) -> Result<Vec<PimdirFlagged>> {
        let staged = self.stage_flag_op(mailbox, ids, flags, op)?;

        for error in staged.errors {
            let id = error.id.unwrap_or_default();
            warn!("Message `{id}` not found in `{mailbox}`, skipped");
        }

        Ok(staged.value)
    }

    /// Stages a flag op on the messages of `ids` the mailbox holds,
    /// returning the queue row each message's `SetFlags` waits in, `None`
    /// for one the op leaves as it shows, for which nothing is staged.
    ///
    /// An id the mailbox does not hold is an item error, so the caller
    /// reports how many messages it found. The rows are appended as one batch
    /// (pimdir STORAGE §15.1), each passing the gate of the sources'
    /// declarations (§15.6) once: a change the gate refuses on any
    /// message, or the store failing, leaves nothing staged. A partial
    /// support passes, its warning logged once the batch is queued.
    fn stage_flag_op(
        &mut self,
        mailbox: &str,
        ids: &[&str],
        flags: &[Flag],
        op: FlagOp,
    ) -> Result<Outcome<Vec<PimdirFlagged>>> {
        let collection = self.hub_id(mailbox)?;

        let mut staged = Vec::with_capacity(ids.len());
        let mut changes = Vec::with_capacity(ids.len());
        let mut errors = Vec::new();
        for id in ids {
            let Some(item) = self.get(&collection, id)? else {
                errors.push(missing_message(id));
                continue;
            };
            let replacement = apply_flag_op(&item.flags, flags, op)
                .with_context(|| format!("Stage flags on `{id}` in `{mailbox}`"))?;
            let changed = replacement != item.flags;
            if changed {
                // NOTE: a set replaces a set it never read, where an add
                // or a remove changes the one read here.
                let base = match op {
                    FlagOp::Set => None,
                    FlagOp::Add | FlagOp::Remove => Some(item.flags),
                };
                let action = PimdirAction::SetFlags {
                    seq: item.seq,
                    flags: replacement,
                    base,
                };
                changes.push((*id, action));
            }
            staged.push((*id, changed));
        }

        let rows = self.enqueue_all(&collection, &changes, |id| match id {
            Some(id) => format!("Stage flags on `{id}` in `{mailbox}`"),
            None => format!("Stage flags in `{mailbox}`"),
        })?;
        let mut rows = rows.into_iter();

        let staged = staged
            .into_iter()
            .map(|(id, changed)| {
                let row = changed.then(|| rows.next()).flatten();
                let partial: Vec<String> = row
                    .iter()
                    .flat_map(|row| &row.partial)
                    .map(ToString::to_string)
                    .collect();
                for partial in &partial {
                    warn!("{partial}");
                }
                PimdirFlagged {
                    id: id.to_owned(),
                    queue_id: row.map(|row| row.handle),
                    partial,
                }
            })
            .collect();

        Ok(Outcome {
            value: staged,
            errors,
        })
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
            .enqueue_with_body(&collection, &raw, |hash| PimdirAction::Add {
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
            queue_id: enqueued.handle,
            link_id: link_id.0,
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
            .enqueue_with_body(&collection, &raw, |_| action)
            .map_err(pimdir_error)
            .with_context(|| format!("Queue send in `{mailbox}`"))?;
        for partial in &enqueued.partial {
            warn!("{partial}");
        }
        let handle = enqueued.handle;
        info!("message queued for sending as action {handle}, see `himalaya pimdir queue list`");
        Ok(PimdirSubmitted {
            staged: PimdirStaged {
                queue_id: handle,
                link_id: mail::derive(&raw).link_id.0,
            },
            carried,
        })
    }

    /// Where the queue row `handle` names stands (pimdir STORAGE §15.4):
    /// pending, parked, unconfirmed, applied with the item an `add`
    /// created, unknown, or reset when the handle names another store. A
    /// row anchored on a collection that is not one of the account's
    /// mailboxes, another account's or another app's, reads as unknown.
    ///
    /// A row applied on a collection gone since (§15.4) names none, so no
    /// account can be told from it and it reads as applied.
    pub fn queue_row(&self, handle: &PimdirActionHandle) -> Result<PimdirActionStatus> {
        let status = self
            .store
            .action_status(handle)
            .map_err(pimdir_error)
            .with_context(|| format!("Read queue row {handle}"))?;
        let collection = match &status {
            PimdirActionStatus::Pending { collection, .. }
            | PimdirActionStatus::Parked { collection, .. }
            | PimdirActionStatus::Unconfirmed { collection, .. }
            | PimdirActionStatus::Applied {
                collection: Some(collection),
                ..
            } => collection,
            PimdirActionStatus::Applied {
                collection: None, ..
            }
            | PimdirActionStatus::Unknown
            | PimdirActionStatus::Reset => return Ok(status),
        };

        let ours = self
            .mail_collections()?
            .iter()
            .any(|ours| &ours.id == collection);
        Ok(if ours {
            status
        } else {
            PimdirActionStatus::Unknown
        })
    }

    /// The row `handle` names as `pimdir queue list` prints it, while
    /// `status` holds it queued; `None` once applied or gone.
    pub fn listed_row(
        &self,
        handle: &PimdirActionHandle,
        status: &PimdirActionStatus,
    ) -> Result<Option<PimdirQueueRow>> {
        let rows = match status {
            PimdirActionStatus::Pending { collection, .. } => self.queued_rows(collection)?.value,
            PimdirActionStatus::Parked { collection, .. }
            | PimdirActionStatus::Unconfirmed { collection, .. } => self.parked_rows(collection)?,
            _ => return Ok(None),
        };

        Ok(rows.into_iter().find(|row| row.row == handle.id))
    }

    /// Copies each id from `from` to `to`, staged as `Copy` (a server-side copy
    /// on the next sync, no body re-upload).
    pub fn copy_messages(&mut self, from: &str, to: &str, ids: &[&str]) -> Result<Outcome<usize>> {
        self.refile(from, to, ids, |seq, target| PimdirAction::Copy {
            seq,
            to: target,
        })
    }

    /// Moves each id between two mailboxes, staged as one server-side move
    /// for the next sync.
    ///
    /// A move states no resolution (pimdir SYNC §7): the owner parks it
    /// over a message a sync left in conflict, as it does a remove, and
    /// the message stays listed where it was (STORAGE §15.4). A move onto
    /// its own mailbox stages nothing, counting the ids it holds.
    pub fn move_messages(&mut self, from: &str, to: &str, ids: &[&str]) -> Result<Outcome<usize>> {
        let source = self.hub_id(from)?;
        if source == self.hub_id(to)? {
            return Ok(self.found(&source, ids)?.map(|found| found.len()));
        }

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
    ///
    /// A mailbox the performer's source already syncs under that name, or
    /// whose creation is already queued, is refused with `mailbox-duplicate`
    /// before anything is queued.
    pub fn create_mailbox(
        &mut self,
        name: &str,
        parent: Option<&str>,
    ) -> Result<PimdirQueuedIntent> {
        // NOTE: a name the store does not list yet is the server's to
        // refuse: the row parks, and `pimdir queue show` says why.
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
            return Err(missing(
                "The sources of this store declare no capabilities, so its sync engine \
                 cannot create a mailbox",
            ));
        };

        if let Some(held) = self.held_mailbox(&source, name, parent.as_deref())? {
            let cause = anyhow!("The pimdir store lists it as `{held}`");
            return Err(existing_mailbox(name, cause));
        }
        if self.creation_queued(&anchor, &source, name, parent.as_deref())? {
            let cause = anyhow!("The pimdir store has its creation queued");
            return Err(existing_mailbox(name, cause));
        }

        let enqueued = producer
            .enqueue_collection_create(&anchor, name, parent.as_deref(), Some(&source))
            .map_err(pimdir_error)
            .with_context(|| format!("Queue the creation of `{name}`"))?;
        Ok(PimdirQueuedIntent::new(enqueued, source, None))
    }

    /// The mail collection of the account `source` syncs that creating
    /// `name` under `parent` would make, if the store lists one: at the
    /// top, one named `name`; under a parent, one named the parent's name,
    /// a hierarchy delimiter (`/` or `.`, the store recording none) and
    /// `name`. Names are the server's own, compared exactly.
    fn held_mailbox(
        &self,
        source: &str,
        name: &str,
        parent: Option<&str>,
    ) -> Result<Option<String>> {
        let collections = self.mail_collections()?;
        let names: Vec<String> = match parent {
            None => vec![name.to_owned()],
            Some(parent) => {
                let parent = collections
                    .iter()
                    .find(|collection| collection.id == parent)
                    .map_or(parent, |collection| collection.name.as_str());
                ['/', '.']
                    .iter()
                    .map(|delimiter| format!("{parent}{delimiter}{name}"))
                    .collect()
            }
        };

        for collection in collections {
            if !names.contains(&collection.name) {
                continue;
            }
            let sources = self
                .store
                .collection_sources(&collection.id)
                .map_err(pimdir_error)
                .with_context(|| format!("Read the sources of `{}`", collection.id))?;
            if sources.iter().any(|synced| synced == source) {
                return Ok(Some(collection.id));
            }
        }

        Ok(None)
    }

    /// Whether a creation of `name` under `parent` by `source` is pending
    /// on `anchor`, where every such creation is queued.
    fn creation_queued(
        &self,
        anchor: &str,
        source: &str,
        name: &str,
        parent: Option<&str>,
    ) -> Result<bool> {
        let pending = self
            .store
            .pending_actions(anchor)
            .map_err(pimdir_error)
            .with_context(|| format!("Read the queue of `{anchor}`"))?;

        Ok(pending.iter().any(|row| match &row.action {
            PimdirAction::Unknown { kind, payload, .. } if kind == "collection-create" => {
                intent::PimdirCollectionCreate::from_payload(payload).is_ok_and(|create| {
                    create.source == source
                        && create.name == name
                        && create.parent.as_ref().map(|parent| parent.as_str()) == parent
                })
            }
            _ => false,
        }))
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
            return Err(missing(
                "The sources of this store declare no capabilities, so its sync engine \
                 cannot rename a mailbox",
            ));
        };

        let enqueued = producer
            .enqueue_collection_rename(&collection, name, Some(&source))
            .map_err(pimdir_error)
            .with_context(|| format!("Queue the renaming of `{collection}` to `{name}`"))?;
        Ok(PimdirQueuedIntent::new(enqueued, source, Some(collection)))
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
            return Err(missing(
                "The sources of this store declare no capabilities, so its sync engine \
                 cannot delete a mailbox",
            ));
        };

        let enqueued = producer
            .enqueue_collection_delete(&collection, Some(&source))
            .map_err(pimdir_error)
            .with_context(|| format!("Queue the deletion of `{collection}`"))?;
        Ok(PimdirQueuedIntent::new(enqueued, source, Some(collection)))
    }

    /// Deletes each id `mailbox` holds, staged as `Remove` (the next sync
    /// pushes it as the backend's own disposal), returning how many it found.
    ///
    /// Every id is checked before the rows are staged, an unknown one
    /// skipped, so the caller reports how many messages it found. The rows
    /// are one batch (pimdir STORAGE §15.1): an id the gate refuses leaves
    /// nothing staged.
    ///
    /// The remove names no body it read and resolves no conflict (pimdir
    /// STORAGE §15.3): it deletes whatever the message holds, and the
    /// owner parks it over a message a sync left in conflict, Himalaya
    /// deciding none.
    pub fn delete_messages(&mut self, mailbox: &str, ids: &[&str]) -> Result<Outcome<usize>> {
        let collection = self.hub_id(mailbox)?;
        let Outcome {
            value: found,
            errors,
        } = self.found(&collection, ids)?;
        let actions: Vec<_> = found
            .iter()
            .map(|(id, seq)| {
                let action = PimdirAction::Remove {
                    seq: *seq,
                    if_match: None,
                    resolves: None,
                };
                (*id, action)
            })
            .collect();

        let rows = self.enqueue_all(&collection, &actions, |id| match id {
            Some(id) => format!("Stage delete of `{id}` in `{mailbox}`"),
            None => format!("Stage deletes in `{mailbox}`"),
        })?;
        warn_partials(&rows);
        Ok(Outcome {
            value: found.len(),
            errors,
        })
    }

    /// Stages one refiling action per id `from` holds, `build` deciding
    /// whether the source copy stays, returning how many it found.
    ///
    /// Every id is checked before the rows are staged, an unknown one an
    /// item error, so the caller reports how many messages it found. The
    /// rows are one batch (pimdir STORAGE §15.1): an id the gate refuses
    /// leaves nothing staged.
    fn refile(
        &mut self,
        from: &str,
        to: &str,
        ids: &[&str],
        build: fn(i64, PimdirCollectionId) -> PimdirAction,
    ) -> Result<Outcome<usize>> {
        let source = self.hub_id(from)?;
        let target = PimdirCollectionId(self.hub_id(to)?);
        let Outcome {
            value: found,
            errors,
        } = self.found(&source, ids)?;
        let actions: Vec<_> = found
            .iter()
            .map(|(id, seq)| (*id, build(*seq, target.clone())))
            .collect();

        let rows = self.enqueue_all(&source, &actions, |id| match id {
            Some(id) => format!("Stage refile of `{id}` from `{from}` to `{to}`"),
            None => format!("Stage refiles from `{from}` to `{to}`"),
        })?;
        warn_partials(&rows);
        Ok(Outcome {
            value: found.len(),
            errors,
        })
    }

    /// The item behind a public id, or `None` when the collection holds none.
    fn get(&self, collection: &str, id: &str) -> Result<Option<PimdirItem>> {
        self.store
            .get_item(collection, parse_id(id)?)
            .map_err(pimdir_error)
            .with_context(|| format!("Read `{id}` in `{collection}`"))
    }

    /// The public ids of `ids` the collection holds, checked here so a stale
    /// id is an item error rather than parked by the owner much later; a
    /// malformed one fails.
    fn found<'a>(&self, collection: &str, ids: &[&'a str]) -> Result<Outcome<Vec<(&'a str, i64)>>> {
        let mut found = Outcome::from(Vec::with_capacity(ids.len()));
        for id in ids {
            match self.get(collection, id)? {
                Some(item) => found.value.push((*id, item.seq)),
                None => found.errors.push(missing_message(id)),
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
    /// the mailbox shows (`PimdirReader::overlay_quiet`).
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

    /// Pulls every live item of a collection with its summary by keyset
    /// paging, newest first (the read API is paginated; the unified
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

    /// What the store holds of the account, as `pimdir status` reports
    /// it: its mailboxes by id with how many messages carry a local body
    /// (read off the summaries, no body opened) and how many creations
    /// and sends are queued, the rows parked in them and the refusals
    /// standing there.
    pub fn status(&self) -> Result<PimdirStatusOutput> {
        let accounts = self
            .store
            .list_accounts()
            .map_err(pimdir_error)
            .context("List the pimdir store accounts")?;
        let mut collections = self.mail_collections()?;
        collections.sort_by(|a, b| a.id.cmp(&b.id));

        let mut mailboxes = Vec::new();
        let mut refused = Vec::new();

        for collection in &collections {
            let id = collection.id.as_str();
            refused.extend(
                self.refused_rows(id)?
                    .into_iter()
                    .map(|row| PimdirRefusedStatus {
                        mailbox_id: row.collection,
                        item_id: row.item_id,
                        source: row.source,
                        reason: row.reason,
                        refused_at: row.refused_at,
                    }),
            );

            let total = self
                .store
                .count_items(id)
                .map_err(pimdir_error)
                .with_context(|| format!("Count items in `{id}`"))?;
            let mut hydrated = 0;
            self.walk(id, None, false, SCAN_BATCH, |item| {
                hydrated += u64::from(item.object.is_some());
                ControlFlow::Continue(SCAN_BATCH)
            })?;
            let queued = self
                .store
                .pending_actions(id)
                .map_err(pimdir_error)
                .with_context(|| format!("List queued messages in `{id}`"))?
                .iter()
                .filter(|queued| is_queued_mail(&queued.action))
                .count();

            mailboxes.push(PimdirMailboxStatus {
                id: collection.id.clone(),
                name: match collection.name.is_empty() {
                    true => collection.id.clone(),
                    false => collection.name.clone(),
                },
                total,
                hydrated,
                queued: u64::try_from(queued).unwrap_or_default(),
            });
        }

        let identity = self.store_identity()?;
        let parked = self
            .store
            .parked_actions()
            .map_err(pimdir_error)
            .context("List parked actions")?
            .into_iter()
            .filter(|parked| collections.iter().any(|c| c.id == parked.collection))
            .map(|parked| PimdirParkedStatus {
                row: parked.id,
                handle: handle(&identity, parked.id),
                mailbox_id: parked.collection,
                kind: parked.action,
                error: parked.error,
            })
            .collect();

        Ok(PimdirStatusOutput {
            name: String::new(),
            account: self.account.clone(),
            accounts,
            mailboxes,
            parked,
            refused,
        })
    }
}

/// Whether a queued action is a message waiting for a public id: a create,
/// or a send the owner performs.
/// Logs each capability the sources support in part that the gate of
/// pimdir STORAGE §15.6 passed `rows` with.
fn warn_partials(rows: &[PimdirEnqueued]) {
    for partial in rows.iter().flat_map(|row| &row.partial) {
        warn!("{partial}");
    }
}

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
        subject: Some(mail.subject).filter(|subject| !subject.is_empty()),
        from: mail.from.into_iter().map(address).collect(),
        to: mail.to.into_iter().map(address).collect(),
        date: mail
            .date
            .as_deref()
            .and_then(|date| DateTime::parse_from_rfc3339(date).ok()),
        size: mail.size,
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
    pub queue_id: PimdirActionHandle,
    /// The link id the message is stored under, the id its write's
    /// receipt prints.
    pub link_id: String,
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
    pub queue_id: PimdirActionHandle,
    /// The source that performs it on its server.
    pub source: String,
    /// The mailbox it renames or deletes, `None` for a creation, the new
    /// mailbox's id arriving with the sync.
    pub mailbox: Option<String>,
}

impl PimdirQueuedIntent {
    /// The intent `enqueued` queued for `source` on `mailbox`, warning
    /// about each capability the source supports in part (pimdir STORAGE
    /// §15.6).
    fn new(enqueued: PimdirEnqueued, source: String, mailbox: Option<String>) -> Self {
        for partial in &enqueued.partial {
            warn!("{partial}");
        }
        Self {
            queue_id: enqueued.handle,
            source,
            mailbox,
        }
    }
}

/// A flag change staged on one message for the owner.
#[derive(Clone, Debug)]
pub struct PimdirFlagged {
    /// The message's public id.
    pub id: String,
    /// The `set-flags` row it waits in, `None` when the change left the
    /// message as it shows and nothing was staged.
    pub queue_id: Option<PimdirActionHandle>,
    /// What a source supports of the change only in part, as its
    /// declaration words it: the row is queued, and the server may keep
    /// less than it asks.
    pub partial: Vec<String>,
}

/// What a queue row addresses, as `pimdir queue list` prints it.
#[derive(Clone, Debug, Default)]
struct Addressed {
    /// The link id of the message the row creates.
    link_id: Option<String>,
    /// The public id of the stored message the row acts on.
    item_id: Option<String>,
    /// The subject of the body the row pins, else of the message it acts
    /// on.
    subject: Option<String>,
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
        .map_err(|_| message_not_found(format!("Invalid message id `{id}` (expected a number)")))
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
    use crate::config::pimdir::PimdirConfig;

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
        assert_eq!(envelope.subject.as_deref(), Some("Hi"));
        assert_eq!(envelope.from[0].email, "a@x.org");
        assert_eq!(envelope.from[0].name.as_deref(), Some("Alice"));
        assert_eq!(envelope.to[0].email, "b@x.org");
        assert_eq!(envelope.to[0].name, None);
        assert_eq!(
            envelope.date.map(|date| date.to_rfc3339()).as_deref(),
            Some("2026-08-01T10:00:00+00:00")
        );
        assert_eq!(envelope.size, Some(99));
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
        assert_eq!(envelope.subject, None);
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

    /// A queued add carries no summary, so its row's subject is derived
    /// from the body it pins, the way the owner will derive it; the row
    /// names the link id its receipt printed and no stored message, and
    /// no placeholder stands in for the id the message lacks.
    #[test]
    fn a_queued_creation_names_its_link_id_and_its_subject() {
        let (_dir, mut client) = sent_store();
        let body = b"Message-ID: <draft@x.org>\r\nSubject: Re: lunch\r\n\r\nbody";
        let staged = client.add_message("imap/Sent", &[], body.to_vec()).unwrap();

        let rows = client.queued_rows("imap/Sent").unwrap().value;

        assert_eq!(rows.len(), 1, "{rows:?}");
        assert_eq!(rows[0].row, staged.queue_id.id);
        assert_eq!(rows[0].kind, "add");
        assert_eq!(rows[0].collection, "imap/Sent");
        assert_eq!(rows[0].link_id.as_deref(), Some("draft@x.org"));
        assert_eq!(rows[0].item_id, None);
        assert_eq!(rows[0].subject.as_deref(), Some("Re: lunch"));
        assert_eq!((rows[0].attempts, rows[0].unconfirmed), (0, false));
        assert_eq!(rows[0].error, None);
    }

    /// A pending row whose payload does not decode leaves its mailbox's
    /// rows unread rather than failing the listing: the mailbox is one
    /// item error, coded as io-pimdir codes the payload.
    #[test]
    fn an_undecodable_row_is_an_item_error_of_its_mailbox() {
        let (dir, mut client) = sent_store();
        let staged = client
            .add_message("imap/Sent", &[], DRAFT.to_vec())
            .unwrap();

        let conn = rusqlite::Connection::open(dir.path().join("pimdir.db")).unwrap();
        let update = "UPDATE queue SET payload = '{' WHERE id = ?1";
        conn.execute(update, [staged.queue_id.id]).unwrap();
        drop(conn);

        let queued = client.queued_rows("imap/Sent").unwrap();
        assert!(queued.value.is_empty(), "{:?}", queued.value);
        assert_eq!(queued.errors.len(), 1, "{:?}", queued.errors);
        assert_eq!(queued.errors[0].id.as_deref(), Some("imap/Sent"));
        assert_eq!(queued.errors[0].code, Some(ErrorCode::ActionMalformed));
    }

    /// A row acting on a stored message names it by its public id, its
    /// subject `null` when the store holds no such message.
    #[test]
    fn a_row_acting_on_a_message_names_its_public_id() {
        let (_dir, client) = sent_store();
        let removal = PimdirAction::Remove {
            seq: 42,
            if_match: None,
            resolves: None,
        };

        let addressed = client.addressed("imap/Sent", &removal);

        assert_eq!(addressed.item_id.as_deref(), Some("42"));
        assert_eq!(addressed.link_id, None);
        assert_eq!(addressed.subject, None);
    }

    /// A bare `pimdir queue list` covers the whole account, so a send
    /// queued on the sent mailbox shows there; `--mailbox` narrows it to
    /// one mailbox.
    #[test]
    fn the_queue_list_covers_the_account_unless_narrowed() {
        use clap::Parser;
        use pimalaya_cli::printer::CapturePrinter;

        use crate::{account::context::Account, pimdir::queue::list::PimdirQueueListCommand};

        let (dir, mut client) = sent_store();
        let store = io_pimdir::client::PimdirStore::open(dir.path()).unwrap();
        store.ensure_collection("imap/INBOX", MAIL_KIND).unwrap();
        drop(store);
        syncs(dir.path());
        client
            .send_message(Some("imap/Sent"), RAW.to_vec(), false)
            .unwrap();

        let list = |client: &mut PimdirClient, args: &[&str]| {
            let command = PimdirQueueListCommand::try_parse_from(args).unwrap();
            let mut printer = CapturePrinter::default();
            let mut account = Account::default();
            command.execute(&mut printer, &mut account, client).unwrap();
            printer.into_single().unwrap()
        };

        let all = list(&mut client, &["list"]);
        assert_eq!(all["actions"][0]["kind"], "submit", "{all}");
        assert_eq!(all["actions"][0]["collection"], "imap/Sent", "{all}");

        let inbox = list(&mut client, &["list", "-m", "imap/INBOX"]);
        assert_eq!(inbox["actions"], serde_json::json!([]), "{inbox}");
    }

    /// A store holding one mail collection, `imap/Sent`, and the client
    /// reading it.
    fn sent_store() -> (tempfile::TempDir, PimdirClient) {
        let dir = tempfile::tempdir().unwrap();
        let store = io_pimdir::client::PimdirStore::open(dir.path()).unwrap();
        store.ensure_collection("imap/Sent", MAIL_KIND).unwrap();
        drop(store);

        let config = crate::config::pimdir::PimdirConfig {
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

        // NOTE: a send creates no message in the store, so its row names
        // no link id; its subject is read off the body it pins.
        let queued = client.queued_rows("imap/Sent").unwrap().value;
        assert_eq!(queued.len(), 1);
        assert_eq!(queued[0].row, pending[0].id);
        assert_eq!(queued[0].kind, "submit");
        assert_eq!(queued[0].collection, "imap/Sent");
        assert_eq!(queued[0].link_id, None);
        assert_eq!(queued[0].item_id, None);
        assert_eq!(queued[0].subject.as_deref(), Some("hi"));
        assert_eq!(queued[0].error, None);
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

        assert_eq!(staged.link_id, "d1@x.org");
        let pending = client.store.pending_actions("imap/Sent").unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, staged.queue_id.id);
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
        assert_eq!(pending[0].id, staged.queue_id.id);
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

        assert_eq!(sent.staged.link_id, "d1@x.org");
        let pending = client.store.pending_actions("imap/Sent").unwrap();
        assert_eq!(pending[0].id, sent.staged.queue_id.id);
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
                .queue_id
                .id)
        });
    }

    #[test]
    fn a_sent_body_waits_for_the_collector() {
        assert_stages_after_the_collector(|client| {
            Ok(client
                .send_message(Some("imap/Sent"), DRAFT.to_vec(), false)?
                .staged
                .queue_id
                .id)
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
        } = client.queue_row(&staged.queue_id).unwrap()
        else {
            panic!("expected a pending row");
        };
        assert_eq!(collection, "imap/Sent");
        assert_eq!(kind, "add");

        assert_eq!(client.cancel_queued(&staged.queue_id).unwrap(), "add");
        assert_eq!(
            client.queue_row(&staged.queue_id).unwrap(),
            PimdirActionStatus::Unknown
        );
    }

    /// A receipt names its row under the store's identity (pimdir STORAGE
    /// §4.5), and the bare id `queue list` prints is taken all the same.
    #[test]
    fn a_receipt_handle_names_the_store_and_a_bare_id_still_cancels() {
        let (_dir, mut client) = sent_store();
        let staged = client
            .add_message("imap/Sent", &[], DRAFT.to_vec())
            .unwrap();

        let printed = staged.queue_id.to_string();
        assert!(staged.queue_id.store.is_some(), "{printed}");
        assert_eq!(
            printed.parse::<PimdirActionHandle>().unwrap(),
            staged.queue_id
        );

        let bare = PimdirActionHandle::from(staged.queue_id.id);
        assert_eq!(client.cancel_queued(&bare).unwrap(), "add");
    }

    /// A receipt's row and handle are those `queue list` prints, and
    /// `queue show` given the bare id prints the same pair.
    #[test]
    fn a_listed_row_carries_the_receipt_row_and_handle() {
        use crate::pimdir::queue::show::PimdirQueueShowOutput;

        let (_dir, mut client) = sent_store();
        let staged = client
            .add_message("imap/Sent", &[], DRAFT.to_vec())
            .unwrap();
        let handle = staged.queue_id.to_string();

        let rows = client.queued_rows("imap/Sent").unwrap().value;
        assert_eq!(
            (rows[0].row, rows[0].handle.as_str()),
            (staged.queue_id.id, handle.as_str())
        );

        let bare = PimdirActionHandle::from(staged.queue_id.id);
        let named = client.named(&bare).unwrap();
        let status = client.queue_row(&bare).unwrap();
        let listed = client.listed_row(&named, &status).unwrap();
        let shown = PimdirQueueShowOutput::new(&named, status, listed).unwrap();
        assert_eq!((shown.row, shown.handle), (staged.queue_id.id, handle));
    }

    /// A handle another store answered, a copy restored since, is refused
    /// `store-reset` and the row its id names is left queued.
    #[test]
    fn a_handle_of_another_store_is_refused_as_reset() {
        let (dir, mut client) = sent_store();
        let staged = client
            .add_message("imap/Sent", &[], DRAFT.to_vec())
            .unwrap();
        let mut other = staged.queue_id.clone();
        if let Some(store) = other.store.as_mut() {
            store.incarnation += 1;
        }

        assert_eq!(client.queue_row(&other).unwrap(), PimdirActionStatus::Reset);
        let err = client.cancel_queued(&other).unwrap_err();

        assert_eq!(
            pimalaya_cli::error::code_of::<ErrorCode>(&err),
            Some(ErrorCode::StoreReset),
            "{err:#}"
        );
        let store = io_pimdir::client::PimdirStore::open(dir.path()).unwrap();
        assert_eq!(store.pending_actions("imap/Sent").unwrap().len(), 1);
    }

    /// A row another app queued on a collection of the same account, an
    /// address book here, reads as unknown: it can be neither shown nor
    /// cancelled, as Cardamum and Calendula keep to their own kind.
    #[test]
    fn a_queue_row_of_another_kind_is_unknown() {
        let (dir, client) = sent_store();
        let card = b"BEGIN:VCARD\r\nVERSION:4.0\r\nUID:u1\r\nFN:A\r\nEND:VCARD\r\n";
        let row = {
            let store = io_pimdir::client::PimdirStore::open(dir.path()).unwrap();
            store
                .ensure_collection("carddav/contacts", "text/vcard")
                .unwrap();
            drop(store);
            client
                .producer()
                .unwrap()
                .enqueue_with_body("carddav/contacts", card, |hash| PimdirAction::Add {
                    link_id: Some(PimdirLinkId("u1".into())),
                    flags: Default::default(),
                    object: Some(hash),
                    supersedes: None,
                })
                .unwrap()
                .handle
                .id
        };

        assert_eq!(
            client.queue_row(&row.into()).unwrap(),
            PimdirActionStatus::Unknown
        );
        let err = client.cancel_queued(&row.into()).unwrap_err().to_string();
        assert!(err.contains("No queued action"), "{err}");

        let store = io_pimdir::client::PimdirStore::open(dir.path()).unwrap();
        assert_eq!(store.pending_actions("carddav/contacts").unwrap().len(), 1);
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
        stamp_format(dir.path(), "storage-draft-11");

        let err = client.cancel_queued(&staged.queue_id).unwrap_err();

        assert_eq!(
            pimalaya_cli::error::code_of::<ErrorCode>(&err),
            Some(ErrorCode::StoreVersion),
            "{err:#}"
        );
        assert!(
            format!("{err:#}").contains("move its directory aside"),
            "{err:#}"
        );
        let db = rusqlite::Connection::open(dir.path().join("pimdir.db")).unwrap();
        let (version, queued): (String, i64) = db
            .query_row(
                "SELECT format_version, (SELECT count(*) FROM queue WHERE id = ?1) \
                 FROM store_meta",
                [staged.queue_id.id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(
            version, "storage-draft-11",
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

        let err = client.cancel_queued(&staged.queue_id).unwrap_err();

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

    /// A store of another draft is refused `store-version` in the shared
    /// wording: an earlier one is synced out and set aside, a later one
    /// asks for a later himalaya, and neither is ever deleted.
    #[test]
    fn a_store_of_another_draft_is_refused_saying_which_is_earlier() {
        for (version, asks) in [
            ("storage-draft-11", "move its directory aside"),
            ("storage-draft-15", "upgrade himalaya"),
        ] {
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
            assert!(err.contains(io_pimdir::sql::FORMAT_VERSION), "{err}");
            assert!(err.contains(asks), "{err}");
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

        let err = work.cancel_queued(&staged.queue_id).unwrap_err();

        assert!(err.to_string().contains("account"), "{err}");
        assert_eq!(home.store.pending_actions("home/INBOX").unwrap().len(), 1);
        let ours = work.add_message("work/INBOX", &[], DRAFT.to_vec()).unwrap();
        assert_eq!(work.cancel_queued(&ours.queue_id).unwrap(), "add");
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
                .acknowledge_action(sent.staged.queue_id.id, None)
                .unwrap()
        );
        drop(owner);

        let PimdirActionStatus::Applied {
            collection, seq, ..
        } = client.queue_row(&sent.staged.queue_id).unwrap()
        else {
            panic!("expected a sent row to read applied");
        };
        assert_eq!(collection.as_deref(), Some("imap/Sent"));
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
        } = client.queue_row(&staged.queue_id).unwrap()
        else {
            panic!("expected an applied row");
        };
        assert_eq!(collection.as_deref(), Some("imap/Sent"));
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
        assert_eq!(pending[0].id, created.queue_id.id);
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

    /// A mailbox the performer's source syncs under the name asked, at the
    /// top or under the parent, is refused `mailbox-duplicate` before anything
    /// is queued, as is one whose creation is queued already; another
    /// source's mailbox of that name is no obstacle.
    #[test]
    fn a_mailbox_the_store_lists_is_not_created_again() {
        let (dir, mut client) = sent_store();
        declare(dir.path(), &[capability::COLLECTION_CREATE]);
        for (id, name, source) in [
            ("imap/Sent", "Sent", "imap"),
            ("imap/Projets", "Projets", "imap"),
            ("imap/Sent/2025", "Sent/2025", "imap"),
            ("msgraph/AQMk", "Archives", "msgraph"),
        ] {
            let store = io_pimdir::client::PimdirStore::open(dir.path()).unwrap();
            store.ensure_collection(id, MAIL_KIND).unwrap();
            store.set_collection_name(id, name).unwrap();
            store
                .for_source(source)
                .write(vec![io_pimdir::change::PimdirWriteOp::SetCheckpoint {
                    collection: PimdirCollectionId(id.into()),
                    checkpoint: io_pimdir::collection::PimdirCheckpoint(Vec::new()),
                }])
                .unwrap();
        }

        for (name, parent, held) in [
            ("Projets", None, "imap/Projets"),
            ("2025", Some("imap/Sent"), "imap/Sent/2025"),
        ] {
            let err = client.create_mailbox(name, parent).unwrap_err();
            assert_eq!(
                pimalaya_cli::error::code_of::<ErrorCode>(&err),
                Some(ErrorCode::MailboxDuplicate),
                "{err:#}"
            );
            let sentence = format!("Cannot create mailbox `{name}`: it exists already");
            assert_eq!(err.to_string(), sentence);
            assert!(format!("{err:#}").contains(held), "{err:#}");
        }
        assert!(
            client
                .store
                .pending_actions("msgraph/AQMk")
                .unwrap()
                .is_empty()
        );
        for id in ["imap/Projets", "imap/Sent"] {
            assert!(client.store.pending_actions(id).unwrap().is_empty(), "{id}");
        }

        client.create_mailbox("Archives", None).unwrap();
        let err = client.create_mailbox("Archives", None).unwrap_err();
        assert_eq!(
            pimalaya_cli::error::code_of::<ErrorCode>(&err),
            Some(ErrorCode::MailboxDuplicate),
            "{err:#}"
        );
        client.create_mailbox("2026", Some("imap/Sent")).unwrap();
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
    fn the_store_says_which_writes_its_sources_serve() {
        let (dir, client) = sent_store();

        // NOTE: nothing declared yet, so the static table stands.
        assert_eq!(client.store_support(Capability::FlagsSeen).unwrap(), None);

        declare(
            dir.path(),
            &[capability::MAIL_FLAGS_SEEN, capability::COLLECTION_RENAME],
        );

        let full = |capability| client.store_support(capability).unwrap().unwrap().level;
        assert_eq!(full(Capability::FlagsSeen), CapabilityLevel::Full);
        assert_eq!(full(Capability::MailboxRename), CapabilityLevel::Full);
        assert_eq!(full(Capability::FlagsAnswered), CapabilityLevel::None);
        assert_eq!(full(Capability::MailboxDelete), CapabilityLevel::None);

        // NOTE: a read is the store's own, whatever its sources declare.
        assert_eq!(client.store_support(Capability::MessageRead).unwrap(), None);
    }

    /// A store whose `msgraph` source syncs `msgraph/Drafts` and
    /// `msgraph/Archive` and adds to its drafts folder alone, as neverest
    /// declares a Graph source: `mail.message.add` `none` source-wide,
    /// `full` on the folder of the drafts role.
    fn drafts_only_store() -> (tempfile::TempDir, PimdirClient) {
        use io_pimdir::capability::{PimdirCapability, PimdirSupport};

        let dir = tempfile::tempdir().unwrap();
        let mut store = io_pimdir::client::PimdirStore::open(dir.path())
            .unwrap()
            .for_source("msgraph");
        for id in ["msgraph/Drafts", "msgraph/Archive"] {
            store.ensure_collection(id, MAIL_KIND).unwrap();
            store
                .write(vec![io_pimdir::change::PimdirWriteOp::SetCheckpoint {
                    collection: PimdirCollectionId(id.into()),
                    checkpoint: io_pimdir::collection::PimdirCheckpoint(Vec::new()),
                }])
                .unwrap();
        }
        let add = |collection: Option<&str>, support, detail: Option<&str>| PimdirCapability {
            collection: collection.map(String::from),
            name: capability::MAIL_MESSAGE_ADD.into(),
            support,
            detail: detail.map(String::from),
        };
        let detail = "Drafts only: Graph creates every MIME message as a draft";
        store
            .declare(
                "msgraph",
                &[
                    add(None, PimdirSupport::None, Some(detail)),
                    add(Some("msgraph/Drafts"), PimdirSupport::Full, None),
                ],
            )
            .unwrap();
        drop(store);

        let config = crate::config::pimdir::PimdirConfig {
            root: dir.path().to_path_buf(),
            account: None,
        };
        (dir, PimdirClient::new(config).unwrap())
    }

    /// A collection's own `mail.message.add` row wins over its source's:
    /// an add elsewhere is refused before anything is staged, naming the
    /// mailbox, while the drafts folder takes it.
    #[test]
    fn an_add_where_the_source_adds_nothing_is_refused_up_front() {
        let (_dir, mut client) = drafts_only_store();

        let err = client
            .add_message("msgraph/Archive", &[], DRAFT.to_vec())
            .unwrap_err();

        assert_eq!(
            pimalaya_cli::error::code_of::<ErrorCode>(&err),
            Some(ErrorCode::Unsupported),
            "{err:#}"
        );
        let message = format!("{err:#}");
        assert!(message.contains("msgraph/Archive"), "{message}");
        assert!(message.contains("Drafts only"), "{message}");
        let pending = client.store.pending_actions("msgraph/Archive").unwrap();
        assert!(pending.is_empty());
        let body = client.blobs.path(&client.blobs.hash(DRAFT)).unwrap();
        assert!(!body.exists(), "a refused add left its body behind");

        let staged = client
            .add_message("msgraph/Drafts", &[], DRAFT.to_vec())
            .unwrap();
        let pending = client.store.pending_actions("msgraph/Drafts").unwrap();
        assert_eq!(pending[0].id, staged.queue_id.id);
    }

    #[test]
    fn the_annex_b_names_are_pimdirs() {
        assert_eq!(
            Capability::MailboxCreate.annex_b_name(),
            Some(capability::COLLECTION_CREATE)
        );
        assert_eq!(
            Capability::MessageTrash.annex_b_name(),
            Some(capability::MAIL_MESSAGE_MOVE)
        );
        assert_eq!(
            Capability::FlagsJunk.annex_b_name(),
            Some(capability::MAIL_FLAGS_JUNK)
        );
        assert_eq!(
            Capability::SubmitCopy.annex_b_name(),
            Some(capability::MAIL_SUBMIT_COPY)
        );
    }

    #[test]
    fn a_mailbox_renaming_is_one_intent_on_the_mailbox() {
        let (dir, mut client) = sent_store();
        declare(dir.path(), &[capability::COLLECTION_RENAME]);

        let renamed = client.rename_mailbox("imap/Sent", "Envoyés").unwrap();

        assert_eq!(renamed.source, "imap");
        let (id, kind, payload) = intent(&client);
        assert_eq!(id, renamed.queue_id.id);
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
        assert_eq!(id, deleted.queue_id.id);
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
        assert!(owner.acknowledge_action(renamed.queue_id.id, None).unwrap());
        owner
            .rename_collection("imap/Sent", "imap/Envoyés")
            .unwrap();
        drop(owner);

        let PimdirActionStatus::Applied {
            collection, seq, ..
        } = client.queue_row(&renamed.queue_id).unwrap()
        else {
            panic!("expected a performed renaming to read applied");
        };
        assert_eq!(collection.as_deref(), Some("imap/Envoyés"));
        assert_eq!(seq, None);
    }

    /// What the sync engine does once the server deleted the mailbox: the
    /// receipt outlives the collection (pimdir STORAGE §15.4), so the row
    /// still reads applied, naming no collection.
    #[test]
    fn a_performed_deletion_reads_applied_once_the_mailbox_left() {
        let (dir, mut client) = sent_store();
        declare(dir.path(), &[capability::COLLECTION_DELETE]);
        let deleted = client.delete_mailbox("imap/Sent").unwrap();

        let mut owner = io_pimdir::client::PimdirStore::open(dir.path()).unwrap();
        assert!(owner.acknowledge_action(deleted.queue_id.id, None).unwrap());
        assert!(matches!(
            client.queue_row(&deleted.queue_id).unwrap(),
            PimdirActionStatus::Applied { .. }
        ));
        assert!(owner.delete_collection("imap/Sent").unwrap());
        drop(owner);

        assert!(matches!(
            client.queue_row(&deleted.queue_id).unwrap(),
            PimdirActionStatus::Applied {
                collection: None,
                ..
            }
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
        (Some(3), Some(100)),
        (Some(1), Some(300)),
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
            let (listed, _) = client
                .list_envelopes_counted(mailbox, *page, *size, false)
                .unwrap();
            assert_eq!(
                listed.more,
                page_follows(scanned.len() as u64, *page, *size),
                "list page {page:?} size {size:?}",
            );
            assert_eq!(
                serde_json::to_string(&listed.envelopes).unwrap(),
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
                assert_eq!(
                    (pushed.more, searched.more),
                    (scanned.more, scanned.more),
                    "search `{query}` page {page:?} size {size:?}",
                );
                let scanned = serde_json::to_string(&scanned.envelopes).unwrap();
                assert_eq!(
                    serde_json::to_string(&pushed.envelopes).unwrap(),
                    scanned,
                    "search `{query}` page {page:?} size {size:?}",
                );
                assert_eq!(serde_json::to_string(&searched.envelopes).unwrap(), scanned);
            }
        }
    }

    #[test]
    fn paged_reads_agree_with_the_whole_mailbox_read() {
        let (_dir, mut client) = seeded_store();
        assert!(client.store.overlay_quiet("imap/INBOX").unwrap());
        assert_paged_reads_agree(&mut client, "imap/INBOX");

        // NOTE: a sanity check that the seed holds what the queries reach.
        let unread = "not flag seen".parse::<SearchEmailsQuery>().unwrap();
        let hits = client
            .search_envelopes("imap/INBOX", Some(&unread), None, None, false)
            .unwrap()
            .envelopes;
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

        assert!(!client.store.overlay_quiet("imap/INBOX").unwrap());
        assert!(!client.store.overlay_quiet("imap/Archive").unwrap());
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
        assert!(client.store.overlay_quiet("imap/INBOX").unwrap());
        assert!(!client.store.overlay_quiet("imap/Archive").unwrap());

        client
            .copy_messages("imap/Archive", "imap/Work", &[id.as_str()])
            .unwrap();
        assert!(!client.store.overlay_quiet("imap/Work").unwrap());
        assert!(client.store.overlay_quiet("imap/INBOX").unwrap());
    }

    /// The counts of a store spelling `\Seen` one way only, the store's
    /// unread count reading that spelling alone.
    #[test]
    fn mailbox_counts_read_the_queue_as_the_listing_does() {
        let dir = tempfile::tempdir().unwrap();
        let canonical = |prefix: &str, n: usize| -> Vec<Seeded> {
            mails(prefix, n)
                .into_iter()
                .filter(|mail| !mail.flags.contains("\\seen"))
                .collect()
        };
        seed(dir.path(), "imap/INBOX", &canonical("in", 60));
        seed(dir.path(), "imap/Archive", &canonical("ar", 20));
        seed(dir.path(), "imap/Work", &canonical("wo", 5));
        let mut client = PimdirClient::new(PimdirConfig {
            root: dir.path().to_path_buf(),
            account: None,
        })
        .unwrap();

        let unread: Vec<String> = client
            .list_envelopes("imap/INBOX", None, None, false)
            .unwrap()
            .into_iter()
            .filter(|envelope| envelope.flags.is_empty())
            .map(|envelope| envelope.id)
            .filter(|id| client.read_flags("imap/INBOX", id).unwrap().is_some())
            .take(4)
            .collect();
        let unread: Vec<&str> = unread.iter().map(String::as_str).collect();
        let seen = [Flag::from_iana(IanaFlag::Seen)];
        client
            .store_flags("imap/INBOX", &unread[..2], &seen, FlagOp::Add)
            .unwrap();
        client
            .move_messages("imap/INBOX", "imap/Archive", &unread[2..])
            .unwrap();

        let listed = |client: &mut PimdirClient, mailbox: &str| {
            let envelopes = client.list_envelopes(mailbox, None, None, false).unwrap();
            let unread = envelopes
                .iter()
                .filter(|envelope| {
                    !envelope
                        .flags
                        .iter()
                        .any(|f| f.iana() == Some(IanaFlag::Seen))
                })
                .count() as u64;
            (Some(envelopes.len() as u64), Some(unread))
        };
        let expected: Vec<_> = ["imap/Archive", "imap/INBOX", "imap/Work"]
            .into_iter()
            .map(|mailbox| (mailbox.to_owned(), listed(&mut client, mailbox)))
            .collect();

        let counted: Vec<_> = client
            .list_mailboxes(true)
            .unwrap()
            .into_iter()
            .map(|mailbox| (mailbox.id, (mailbox.total, mailbox.unread)))
            .collect();
        assert_eq!(counted, expected);

        let covered: Vec<_> = client
            .list_covered_mailboxes()
            .unwrap()
            .into_iter()
            .map(|covered| covered.mailbox)
            .map(|mailbox| (mailbox.id, (mailbox.total, mailbox.unread)))
            .collect();
        assert_eq!(covered, expected);
    }

    /// A row filed under another key than its date sends the search back
    /// to the whole-mailbox read, which answers as before. io-pimdir files
    /// every row it writes under its date, so the row is filed elsewhere as
    /// a writer of its own could.
    #[test]
    fn a_row_out_of_date_order_searches_the_whole_mailbox() {
        let dir = tempfile::tempdir().unwrap();
        let seeded = mails("in", 40);
        seed(dir.path(), "imap/INBOX", &seeded);
        let db = rusqlite::Connection::open(dir.path().join("pimdir.db")).unwrap();
        let moved = db
            .execute(
                "UPDATE items SET sort_key = '2026-05-12T10:00:00+02:00' WHERE link_id = 'in1@x.test'",
                [],
            )
            .unwrap();
        assert_eq!(moved, 1);
        drop(db);
        let mut client = PimdirClient::new(PimdirConfig {
            root: dir.path().to_path_buf(),
            account: None,
        })
        .unwrap();

        for query in ["", "after 2026-05-11", "not flag seen order by from"] {
            let parsed = (!query.is_empty()).then(|| query.parse::<SearchEmailsQuery>().unwrap());
            let filter = parsed.as_ref().and_then(|q| q.filter.as_ref());
            let sort = parsed.as_ref().and_then(|q| q.sort.as_deref());
            assert!(
                client
                    .search_pushed("imap/INBOX", filter, sort, None, None)
                    .unwrap()
                    .is_none(),
                "`{query}`",
            );
            let scanned = client
                .search_scanned("imap/INBOX", filter, sort, Some(1), Some(10))
                .unwrap();
            let searched = client
                .search_envelopes("imap/INBOX", parsed.as_ref(), Some(1), Some(10), false)
                .unwrap();
            assert_eq!(searched.more, scanned.more);
            assert_eq!(
                serde_json::to_string(&searched.envelopes).unwrap(),
                serde_json::to_string(&scanned.envelopes).unwrap(),
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
                .envelopes
                .into_iter()
                .map(|envelope| envelope.subject.unwrap_or_default())
                .collect();
            let mut other: Vec<String> = scanned
                .envelopes
                .into_iter()
                .map(|e| e.subject.unwrap_or_default())
                .collect();
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
            Some((u32::MAX as usize).saturating_mul(u32::MAX as usize))
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
                    .envelopes
                    .len()
            });
            let pushed = median(|| {
                client
                    .search_envelopes("imap/INBOX", Some(&parsed), Some(1), Some(50), false)
                    .unwrap()
                    .envelopes
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
            let counted =
                median(|| client.count_mail(parsed.clone(), &options.clock).unwrap().0 as usize);
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

    /// A flag change applied, then refused for good by the server it was
    /// pushed to, reads applied on its message with the refusal in `pimdir
    /// queue show`, and lists beside the mailbox's queue (pimdir STORAGE
    /// §15.4).
    #[test]
    fn a_refused_push_shows_on_its_row_and_beside_the_queue() {
        use io_pimdir::{
            change::{PimdirChangeKey, PimdirWriteOp},
            collection::PimdirCollectionId,
            placement::PimdirHandle,
        };

        use crate::pimdir::queue::show::PimdirQueueShowOutput;

        let (dir, mut client) = flag_store(None);
        let id = id_of(&mut client, 0);
        let staged = client
            .stage_flags(
                "imap/INBOX",
                &[id.as_str()],
                &[keyword("todo")],
                FlagOp::Add,
            )
            .unwrap();
        let row = staged[0].queue_id.clone().unwrap();

        let mut owner = io_pimdir::client::PimdirStore::open(dir.path())
            .unwrap()
            .for_source("imap");
        assert_eq!(owner.drain().unwrap().applied, 1);
        owner
            .write(vec![PimdirWriteOp::RefusePlacement {
                collection: PimdirCollectionId::from("imap/INBOX"),
                handle: PimdirHandle("k0".into()),
                key: PimdirChangeKey::from("892471aec94fc225"),
                reason: String::from("NO [CANNOT] keyword refused"),
            }])
            .unwrap();
        drop(owner);

        let show = PimdirQueueShowOutput::new(&row, client.queue_row(&row).unwrap(), None).unwrap();
        let json = serde_json::to_value(&show).unwrap();
        assert_eq!(json["state"], "applied", "{json}");
        assert_eq!(json["itemId"], id.as_str(), "{json}");
        assert_eq!(json["refusals"][0]["source"], "imap", "{json}");
        assert_eq!(json["refusals"][0]["collection"], "imap/INBOX", "{json}");
        let reason = "NO [CANNOT] keyword refused";
        assert_eq!(json["refusals"][0]["reason"], reason, "{json}");
        let text = show.to_string();
        assert!(text.contains("Refused by imap in imap/INBOX at "), "{text}");

        let refused = client.refused_rows("imap/INBOX").unwrap();
        assert_eq!(refused.len(), 1);
        assert_eq!(refused[0].item_id, id);
        assert_eq!(refused[0].source, "imap");
        assert_eq!(refused[0].reason, reason);
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
        assert_eq!(
            Some(pending[0].id),
            staged[0].queue_id.as_ref().map(|row| row.id)
        );
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

    /// A keyword a source keeps only in part is queued, and the receipt
    /// says so with the source's own words rather than the log alone.
    #[test]
    fn a_keyword_kept_in_part_is_queued_with_its_warning() {
        use io_pimdir::capability::{MAIL_FLAGS_KEYWORDS, PimdirSupport};

        use crate::pimdir::flag::PimdirFlagStageOutput;

        let (_dir, mut client) = flag_store(Some(&[(MAIL_FLAGS_KEYWORDS, PimdirSupport::Partial)]));
        let ids = [id_of(&mut client, 0), id_of(&mut client, 1)];
        let ids: Vec<&str> = ids.iter().map(String::as_str).collect();

        let staged = client
            .stage_flags("imap/INBOX", &ids, &[keyword("todo")], FlagOp::Add)
            .unwrap();

        let warning = "Source imap supports mail.flags.keywords in part: \
                       mail.flags.keywords as declared";
        assert_eq!(staged[0].partial, [warning]);
        let output = PimdirFlagStageOutput::from(staged);
        let json = serde_json::to_value(&output).unwrap();
        assert_eq!(json["warnings"], serde_json::json!([warning]));
        let text = output.to_string();
        assert!(text.contains(&format!("\nWarning: {warning}\n")), "{text}");
    }

    /// A read asked to mark the message seen fails, as on Maildir and
    /// mbox, when the store cannot stage the flag, rather than read it
    /// unseen and exit 0.
    #[test]
    fn a_read_whose_seen_flag_is_refused_fails() {
        use io_pimdir::capability::{MAIL_FLAGS_SEEN, PimdirSupport};

        let (dir, mut client) = sent_store();
        client
            .add_message("imap/Sent", &[], DRAFT.to_vec())
            .unwrap();
        let mut owner = io_pimdir::client::PimdirStore::open(dir.path())
            .unwrap()
            .for_source("imap");
        assert_eq!(owner.drain().unwrap().applied, 1);
        let seen = io_pimdir::capability::PimdirCapability {
            collection: None,
            name: MAIL_FLAGS_SEEN.to_string(),
            support: PimdirSupport::None,
            detail: None,
        };
        owner.declare("imap", &[seen]).unwrap();
        drop(owner);
        syncs(dir.path());
        let envelopes = client.list_envelopes("imap/Sent", None, None, false);
        let id = envelopes.unwrap()[0].id.clone();

        let err = client.get_message("imap/Sent", &id, true).unwrap_err();

        assert_eq!(
            pimalaya_cli::error::code_of::<ErrorCode>(&err),
            Some(ErrorCode::Unsupported),
            "{err:#}"
        );
        assert!(client.get_message("imap/Sent", &id, false).is_ok());
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

    /// The unified `flag set -f seen` on three mails, which drops `act` from
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

    /// An add or a remove states the set it was read from (pimdir STORAGE
    /// §15.3), so the owner applies the change alone: a flag a sync lands
    /// on the message while the row waits in the queue survives the
    /// drain. A set states none and replaces whatever the message holds.
    #[test]
    fn a_flag_synced_while_a_change_waits_survives_all_but_a_set() {
        let cases = [
            (
                0,
                FlagOp::Add,
                "act",
                &["\\Seen"][..],
                &["\\Seen", "act"][..],
            ),
            (2, FlagOp::Remove, "\\Seen", &["\\Seen", "act"], &["act"]),
            (0, FlagOp::Set, "act", &["\\Seen"], &["act"]),
        ];

        for (n, op, flag, synced, expected) in cases {
            let (dir, mut client) = flag_store(None);
            let id = id_of(&mut client, n);
            client
                .store_flags("imap/INBOX", &[id.as_str()], &[keyword(flag)], op)
                .unwrap();

            // NOTE: a sync lands the server's set while the row waits.
            let landed = Seeded {
                handle: format!("k{n}"),
                flags: known(synced),
                summary: Some(PimdirMailSummary {
                    message_id: Some(format!("k{n}@x.test")),
                    subject: format!("keyword {n}"),
                    date: Some(format!("2026-05-1{n}T10:00:00Z")),
                    ..Default::default()
                }),
            };
            seed(dir.path(), "imap/INBOX", &[landed]);

            let mut owner = io_pimdir::client::PimdirStore::open(dir.path())
                .unwrap()
                .for_source("imap");
            let report = owner.drain().unwrap();
            assert_eq!((report.applied, report.parked), (1, 0), "{op:?}");
            drop(owner);

            let mut held = client.read_flags("imap/INBOX", &id).unwrap().unwrap();
            held.sort();
            assert_eq!(held, expected, "{op:?}");
        }
    }

    /// An id the mailbox does not hold is an item error and counted out, as
    /// the unified commands report how many of the given ids they found.
    #[test]
    fn an_unknown_id_is_an_item_error_and_counted_out() {
        let (_dir, mut client) = flag_store(None);
        let id = id_of(&mut client, 0);
        let seen = [Flag::from_iana(IanaFlag::Seen)];

        let stored = client
            .store_flags("imap/INBOX", &[id.as_str(), "999"], &seen, FlagOp::Add)
            .unwrap();
        assert_eq!(stored.value, 1);
        assert_eq!(stored.errors[0].id.as_deref(), Some("999"));
        assert_eq!(stored.errors[0].code, Some(ErrorCode::MessageNotFound));
        assert_eq!(client.store.pending_actions("imap/INBOX").unwrap().len(), 1);

        let staged = client
            .stage_flags("imap/INBOX", &["998", "999"], &seen, FlagOp::Add)
            .unwrap();
        assert!(staged.is_empty());
        assert_eq!(client.store.pending_actions("imap/INBOX").unwrap().len(), 1);
    }

    /// A refile or a delete stages the ids the mailbox holds and counts them,
    /// an unknown one an item error; none known stages nothing, nor does a
    /// move onto its own mailbox.
    #[test]
    fn a_refile_or_delete_stages_the_ids_it_found() {
        let (dir, mut client) = flag_store(None);
        seed(dir.path(), "imap/Archive", &[]);
        let ids: Vec<String> = (0..3).map(|n| id_of(&mut client, n)).collect();
        let queued = |client: &PimdirClient| client.store.list_pending_actions().unwrap().len();

        let copied = client.copy_messages("imap/INBOX", "imap/Archive", &[&ids[0], "999"]);
        let copied = copied.unwrap();
        assert_eq!(copied.value, 1);
        assert_eq!(copied.errors[0].id.as_deref(), Some("999"));
        assert_eq!(queued(&client), 1);
        let moved = client.move_messages("imap/INBOX", "imap/Archive", &[&ids[1], "999"]);
        assert_eq!(moved.unwrap().value, 1);
        assert_eq!(queued(&client), 2);
        let stayed = client.move_messages("imap/INBOX", "imap/INBOX", &[&ids[2], "999"]);
        let stayed = stayed.unwrap();
        assert_eq!(stayed.value, 1);
        assert_eq!(stayed.errors[0].id.as_deref(), Some("999"));
        assert_eq!(queued(&client), 2);
        let deleted = client.delete_messages("imap/INBOX", &[&ids[2], "999"]);
        assert_eq!(deleted.unwrap().value, 1);
        assert_eq!(queued(&client), 3);

        let none = client.copy_messages("imap/INBOX", "imap/Archive", &["998", "999"]);
        let none = none.unwrap();
        assert_eq!(none.value, 0);
        assert_eq!(none.errors.len(), 2);
        assert!(
            client
                .delete_messages("imap/INBOX", &[&ids[0], "x"])
                .is_err()
        );
        assert_eq!(queued(&client), 3);
    }

    /// A copy of several ids is one queue transaction: the gate refusing
    /// the second id leaves the first unstaged too, so a rerun stages it
    /// once.
    #[test]
    fn a_refile_refused_on_one_id_stages_none() {
        use io_pimdir::capability::{
            MAIL_FLAGS_KEYWORDS, MAIL_FLAGS_SEEN, MAIL_MESSAGE_COPY, PimdirSupport,
        };

        let (dir, mut client) = flag_store(Some(&[
            (MAIL_FLAGS_SEEN, PimdirSupport::Full),
            (MAIL_MESSAGE_COPY, PimdirSupport::Full),
            (MAIL_FLAGS_KEYWORDS, PimdirSupport::None),
        ]));
        seed(dir.path(), "imap/Archive", &[]);
        let ids: Vec<String> = (0..3).map(|n| id_of(&mut client, n)).collect();
        let ids: Vec<&str> = ids.iter().map(String::as_str).collect();
        let queued = |client: &PimdirClient| client.store.list_pending_actions().unwrap().len();

        let copied = client.copy_messages("imap/INBOX", "imap/Archive", &ids);
        let err = format!("{:#}", copied.unwrap_err());
        assert!(err.contains(&format!("`{}`", ids[1])), "{err}");
        assert_eq!(queued(&client), 0);

        let copied = client.copy_messages("imap/INBOX", "imap/Archive", &[ids[0], ids[2]]);
        assert_eq!(copied.unwrap().value, 2);
        assert_eq!(queued(&client), 2);
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

    /// A read of an id the mailbox does not hold is `message-not-found`.
    #[test]
    fn a_read_of_an_unknown_id_is_not_found() {
        let (_dir, mut client) = sent_store();

        let err = client.get_message("imap/Sent", "999", false).unwrap_err();

        assert_eq!(
            pimalaya_cli::error::code_of::<ErrorCode>(&err),
            Some(ErrorCode::MessageNotFound),
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

        assert_eq!(count.value, 2);
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
        assert_eq!(parked[0].id, twin.queue_id.id);
        assert_eq!(parked[0].action, "add");
        assert!(!parked[0].error.is_empty());

        // NOTE: the parked add keeps the link id its receipt printed and
        // the subject of the body it pins, in the pending row's shape.
        let rows = client.parked_rows("imap/Sent").unwrap();
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert_eq!(
            (rows[0].row, rows[0].kind.as_str()),
            (twin.queue_id.id, "add")
        );
        assert_eq!(rows[0].link_id.as_deref(), Some(twin.link_id.as_str()));
        assert_eq!(rows[0].item_id, None);
        assert_eq!(rows[0].subject.as_deref(), Some("hi"));
        assert_eq!(rows[0].error.as_deref(), Some(parked[0].error.as_str()));
        assert!(!rows[0].unconfirmed);

        assert_eq!(client.cancel_queued(&twin.queue_id).unwrap(), "add");
        assert!(client.parked_actions("imap/Sent").unwrap().is_empty());
    }

    /// A store whose `imap/Drafts` holds one draft a sync left in conflict
    /// beside an empty `imap/Trash`, the client reading it, and the hash
    /// of the diverging body.
    fn conflicted_store() -> (tempfile::TempDir, PimdirClient, PimdirHash) {
        use io_pimdir::{
            change::PimdirWriteOp,
            object::PimdirObject,
            placement::{PimdirBase, PimdirHandle, PimdirPlacement, PimdirStatus},
        };

        const THEIRS: &[u8] = b"Message-ID: <d1@x.org>\r\nSubject: hi\r\n\r\nhello there";

        let dir = tempfile::tempdir().unwrap();
        let mut store = io_pimdir::client::PimdirStore::open(dir.path())
            .unwrap()
            .for_source("imap");
        for id in ["imap/Drafts", "imap/Trash"] {
            store.ensure_collection(id, MAIL_KIND).unwrap();
        }
        let object = |bytes: &[u8]| PimdirObject {
            hash: store.hash(bytes),
            size: bytes.len(),
        };
        let body = |bytes: &[u8]| PimdirWriteOp::StoreObject {
            object: object(bytes),
            body: Some(bytes.to_vec()),
        };
        let (ours, theirs) = (object(DRAFT).hash, object(THEIRS).hash);
        let derived = mail::derive(DRAFT);
        let draft = PimdirPlacement {
            collection: PimdirCollectionId("imap/Drafts".into()),
            handle: PimdirHandle("1".into()),
            link_id: Some(derived.link_id),
            object: Some(ours.clone()),
            level: PimdirLevel::Full,
            summary: derived.summary,
            sort_key: derived.sort_key,
            flags: known(&[]),
            status: PimdirStatus::Conflict,
            conflict_revision: Some("r2".into()),
            conflict_object: Some(theirs.clone()),
            base: Some(PimdirBase {
                flags: known(&[]),
                revision: Some("r1".into()),
                object: Some(ours),
            }),
            origin: None,
        };
        store
            .write(vec![
                body(DRAFT),
                body(THEIRS),
                PimdirWriteOp::UpsertPlacement(draft),
            ])
            .unwrap();
        drop(store);

        let config = crate::config::pimdir::PimdirConfig {
            root: dir.path().to_path_buf(),
            account: None,
        };
        (dir, PimdirClient::new(config).unwrap(), theirs)
    }

    /// A move states no resolution, so the owner parks one queued on a
    /// message a sync left in conflict (pimdir SYNC §7, STORAGE §15.3) and
    /// the reader folds nothing for it (§15.4): the message stays listed
    /// where it was, and its row reads parked, naming the diverging body,
    /// in the mailbox it was queued in.
    #[test]
    fn a_move_over_a_conflicted_message_stays_listed_and_parks() {
        use crate::pimdir::queue::show::{PimdirQueueShowOutput, PimdirQueueState};

        let (dir, mut client, theirs) = conflicted_store();
        let listed = |client: &mut PimdirClient, mailbox: &str| -> Vec<String> {
            client
                .list_envelopes(mailbox, None, None, false)
                .unwrap()
                .into_iter()
                .map(|envelope| envelope.id)
                .collect()
        };
        let drafts = listed(&mut client, "imap/Drafts");
        assert_eq!(drafts.len(), 1);

        let moved = client
            .move_messages("imap/Drafts", "imap/Trash", &[drafts[0].as_str()])
            .unwrap();
        assert_eq!(moved.value, 1, "the move is queued");
        let row = client.store.pending_actions("imap/Drafts").unwrap()[0].id;
        assert_eq!(listed(&mut client, "imap/Drafts"), drafts, "taken out");
        assert!(listed(&mut client, "imap/Trash").is_empty(), "brought in");

        let mut owner = io_pimdir::client::PimdirStore::open(dir.path())
            .unwrap()
            .for_source("imap");
        let report = owner.drain().unwrap();
        drop(owner);
        assert_eq!((report.applied, report.parked), (0, 1));

        let parked = client.parked_actions("imap/Drafts").unwrap();
        assert_eq!(parked.len(), 1, "{parked:?}");
        assert_eq!((parked[0].id, parked[0].action.as_str()), (row, "move"));
        assert!(parked[0].error.contains(&theirs.0), "{}", parked[0].error);
        assert!(client.parked_actions("imap/Trash").unwrap().is_empty());

        // NOTE: the parked move names the message it acts on, as
        // `envelope list` prints it, and that message's subject.
        let rows = client.parked_rows("imap/Drafts").unwrap();
        assert_eq!(rows[0].item_id.as_deref(), Some(drafts[0].as_str()));
        assert_eq!(rows[0].link_id, None);
        assert_eq!(rows[0].subject.as_deref(), Some("hi"));

        let named = client.named(&row.into()).unwrap();
        let status = client.queue_row(&named).unwrap();
        let row_listed = client.listed_row(&named, &status).unwrap();
        let shown = PimdirQueueShowOutput::new(&named, status, row_listed).unwrap();
        assert_eq!(shown.state, PimdirQueueState::Parked);
        assert_eq!(shown.item_id.as_deref(), Some(drafts[0].as_str()));
        assert_eq!(shown.subject.as_deref(), Some("hi"));
        assert_eq!(shown.kind.as_deref(), Some("move"));
        assert_eq!(shown.error.as_deref(), Some(parked[0].error.as_str()));

        let status = client.status().unwrap();
        assert_eq!(status.parked.len(), 1, "{:?}", status.parked);
        assert_eq!(status.parked[0].row, row);
        assert_eq!(status.parked[0].handle, named.to_string());
        assert_eq!(status.parked[0].kind, "move");
        assert_eq!(status.parked[0].mailbox_id, "imap/Drafts");
        let drafts_status = &status.mailboxes[0];
        assert_eq!(drafts_status.id, "imap/Drafts");
        assert_eq!((drafts_status.hydrated, drafts_status.total), (1, 1));
        assert_eq!(status.mailboxes[1].id, "imap/Trash");

        assert_eq!(listed(&mut client, "imap/Drafts"), drafts);
        assert!(listed(&mut client, "imap/Trash").is_empty());
    }
}
