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

use std::io::Write;

use anyhow::{Result, anyhow, bail};
use chrono::DateTime;
use io_pimdir::{
    capability,
    client::{
        blobs::PimdirBlobWriter,
        producer::{PimdirActionStatus, PimdirPendingAction},
        reader::{PimdirCollection, PimdirItem},
    },
    codec::PimdirAction,
    collection::PimdirCollectionId,
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
        mailbox::Mailbox,
        search::{eval, query::SearchEmailsQuery},
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
                role: None,
                total,
                unread: None,
            });
        }
        mailboxes.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(mailboxes)
    }

    /// Lists envelopes from `mailbox`, built from the stored summaries (no
    /// body reads), in the store's newest-first order then paginated.
    pub fn list_envelopes(
        &mut self,
        mailbox: &str,
        page: Option<u32>,
        page_size: Option<u32>,
        _with_attachment: bool,
    ) -> Result<Vec<Envelope>> {
        let collection = self.hub_id(mailbox)?;
        let envelopes: Vec<Envelope> = self
            .scan_items(&collection)?
            .iter()
            .map(envelope_from_item)
            .collect();
        Ok(paginate(envelopes, page, page_size))
    }

    /// Searches envelopes in `mailbox`: builds them from the summaries,
    /// applies the shared filter/sort/paginate. Body clauses cannot match on
    /// an item whose body is not local (no bytes to scan); header/flag
    /// clauses always do.
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
            level: PimdirLevel::Probed,
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
}
