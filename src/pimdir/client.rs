//! # pimdir client
//!
//! The wrapper around io-pimdir's store, blob reader and producer.
//!
//! The store belongs to the sync engine. Reads take the lockless reader
//! role, so a sync in flight neither blocks Himalaya nor is blocked by
//! it, and writes take the shared lock for one enqueue.
//!
//! The reader overlays the queue, so an action this process staged shows
//! on the next read rather than wait for the owner to apply it.

use std::path::PathBuf;

use anyhow::{Context, Result, anyhow, bail};
use io_pimdir::{
    client::{
        PimdirError, PimdirStore,
        blobs::PimdirBlobs,
        producer::{PimdirActionStatus, PimdirProducer},
        reader::PimdirReader,
    },
    codec::PimdirAction,
    object::PimdirObject,
};
use log::warn;
use pimalaya_cli::error::CodedError;

use crate::{
    account::context::Account,
    config::{AccountConfig, Config, PimdirConfig},
    error::{ErrorCode, pimdir_error},
};

/// The producer name recorded on every action this client enqueues, so the
/// owner's queue says which process staged a row.
const PRODUCER: &str = "himalaya";

/// Live pimdir client: an overlaying reader, a blob reader over the same
/// directory, and the account its collections are grouped under.
pub struct PimdirClient {
    pub(crate) store: PimdirReader,
    pub(crate) blobs: PimdirBlobs,
    /// The store directory, reopened as a producer for each staged write.
    pub(crate) root: PathBuf,
    /// The account grouping this client's collections (pimdir STORAGE §9.2), or
    /// `None` in a store holding a single ungrouped account.
    pub(crate) account: Option<String>,
}

impl PimdirClient {
    /// Opens the pimdir store at the configured root, read-only.
    ///
    /// The store has to exist: Himalaya reads a replica a sync populated, so
    /// creating one here would answer a mistyped root with an empty mailbox
    /// list rather than say the path is wrong.
    pub fn new(config: PimdirConfig) -> Result<Self> {
        let root = config.root.clone();

        if !root.join("pimdir.db").exists() {
            return Err(anyhow!(
                "No pimdir store at `{}`; check `pimdir.root`, and run a sync to create one",
                root.display(),
            ));
        }

        // NOTE: reads overlay the queue, so what this client staged shows on
        // the next read rather than on the next sync.
        let store = PimdirReader::open(&root)
            .map_err(pimdir_error)
            .with_context(|| format!("Open pimdir store `{}`", root.display()))?
            .with_pending();

        let account = resolve_account(&store, config.account.clone())?;
        let blobs = PimdirBlobs::open(&root);

        Ok(Self {
            store,
            blobs,
            root,
            account,
        })
    }

    /// Queues `action` through `producer`, warning about each capability
    /// its source supports in part (pimdir STORAGE §15.6), and answers
    /// the row it waits in.
    ///
    /// The enqueue runs the gate of §15.6 itself, once, and answers the
    /// partial supports it passed the action with.
    pub(crate) fn enqueue(
        &self,
        producer: &mut PimdirProducer,
        collection: &str,
        action: &PimdirAction,
        object: Option<&PimdirObject>,
    ) -> Result<i64, PimdirError> {
        let enqueued = producer.enqueue(collection, action, object)?;
        for partial in &enqueued.partial {
            warn!("{partial}");
        }
        Ok(enqueued.handle.id)
    }

    /// Opens a producer for the length of one staging batch.
    ///
    /// A producer holds the store's shared lock, so it is opened for the
    /// writes it stages and dropped as they land.
    pub(crate) fn producer(&self) -> Result<PimdirProducer> {
        let producer = PimdirProducer::open(&self.root, PRODUCER)
            .map_err(pimdir_error)
            .with_context(|| format!("Open pimdir producer `{}`", self.root.display()))?;

        Ok(match &self.account {
            Some(account) => producer.for_account(account),
            None => producer,
        })
    }
}

impl PimdirClient {
    /// Cancels one pending or parked row of the account's queue, returning
    /// its action kind.
    ///
    /// A row parked unconfirmed (pimdir STORAGE §15.2) is cancelled as any
    /// parked row is: its request was sent and may have taken effect, which
    /// cancelling does not undo, so the caller says so before it asks.
    ///
    /// Cancelling is an owner write, and the only retraction a queued
    /// creation has. The role is entered and released inside this call, so
    /// Himalaya never holds a handle that could drain the queue or collect
    /// the store. The row is read first, and one anchored on another
    /// account's collection is refused before the role is taken.
    ///
    /// The store is opened as it stands, never created, migrated nor
    /// reconciled: a store of another draft is refused, as is a damaged
    /// one, both left as they are.
    ///
    /// A sync in flight owns the store, and that is refused at once rather
    /// than waited out: the action is still queued, and may have been applied
    /// by the time the user reads the message.
    pub fn cancel_queued(&self, id: i64) -> Result<String> {
        let kind = match self.queue_row(id)? {
            PimdirActionStatus::Pending { kind, .. }
            | PimdirActionStatus::Parked { kind, .. }
            | PimdirActionStatus::Unconfirmed { kind, .. } => kind,
            PimdirActionStatus::Applied { .. } => bail!("Row {id} was applied already"),
            // NOTE: a bare row id states no store, so io-pimdir follows
            // it unchecked and never answers a reset for it.
            PimdirActionStatus::Unknown | PimdirActionStatus::Reset => bail!(
                "No queued action with row {id} in this account's queue; \
                 it may have been synced already"
            ),
        };

        // NOTE: a store of another draft is refused saying how to reinstall
        // it (`pimdir_error`).
        let mut owner = PimdirStore::open_current(&self.root).map_err(|err| match err {
            PimdirError::Owned(_) => CodedError::new(
                ErrorCode::StoreOwned,
                format!(
                    "A sync is running on `{}`, so the queue cannot be edited; \
                     the action may have been applied already",
                    self.root.display(),
                ),
            )
            .into(),
            err => pimdir_error(err).context(format!("Cancel queued action {id}")),
        })?;

        if !owner
            .drop_action(id)
            .map_err(pimdir_error)
            .with_context(|| format!("Cancel queued action {id}"))?
        {
            bail!("No queued action with row {id}; it may have been synced already");
        }
        Ok(kind)
    }
}

impl PimdirClient {
    /// The source performing an intent `capability` for this account
    /// (pimdir STORAGE §15.6), `None` when the sources syncing `collection`
    /// declare nothing, the case
    /// [`undeclared_performer`](Self::undeclared_performer) names.
    ///
    /// Several candidates and no recorded choice is the user's to settle:
    /// Himalaya never picks one.
    pub(crate) fn performer(
        &self,
        producer: &PimdirProducer,
        collection: &str,
        capability: &str,
    ) -> Result<Option<String>> {
        // NOTE: a collection the sync engine only declared (`neverest sync
        // --declare-only`) has no source syncing it yet, so the account's
        // candidates count too.
        let declared = producer
            .capabilities(collection)
            .map_err(pimdir_error)
            .with_context(|| format!("Read the capabilities of `{collection}`"))?
            .iter()
            .any(|source| source.declared.is_some())
            || !self.performers(capability)?.0.is_empty();
        if !declared {
            return Ok(None);
        }

        match producer.performer(collection, capability, None) {
            Ok(source) => Ok(Some(source)),
            Err(PimdirError::Ambiguous { candidates, .. }) => Err(CodedError::new(
                ErrorCode::AmbiguousPerformer,
                format!(
                    "Several sources can perform {capability} for this account ({}): \
                     choose one with `himalaya pimdir performer {capability} <SOURCE>`",
                    candidates.join(", "),
                ),
            )
            .into()),
            Err(err) => Err(pimdir_error(err)),
        }
    }

    /// The source performing an intent anchored on `collection` when the
    /// sources syncing it declare nothing (pimdir STORAGE §15.6, Annex
    /// B.2): an undeclared source is gated by nothing, and the payload
    /// still names it.
    ///
    /// The one source syncing `collection` is it. None, the collection
    /// being only declared, and several, none of which says what it
    /// performs, are refused rather than guessed.
    pub(crate) fn undeclared_performer(
        &self,
        producer: &PimdirProducer,
        collection: &str,
        capability: &str,
    ) -> Result<String> {
        let sources: Vec<String> = producer
            .capabilities(collection)
            .map_err(pimdir_error)
            .with_context(|| format!("Read the sources of `{collection}`"))?
            .into_iter()
            .map(|source| source.source)
            .collect();

        match sources.as_slice() {
            [source] => Ok(source.clone()),
            [] => Err(CodedError::new(
                ErrorCode::NoPerformer,
                format!(
                    "No source syncs `{collection}` yet, so none performs {capability} \
                     there: run a sync first"
                ),
            )
            .into()),
            _ => Err(CodedError::new(
                ErrorCode::AmbiguousPerformer,
                format!(
                    "Several sources sync `{collection}` ({}) and none declares \
                     {capability}: their sync engine has to declare who performs it",
                    sources.join(", "),
                ),
            )
            .into()),
        }
    }

    /// The sources able to perform `capability` for this account, and the
    /// one the user chose among them, if any.
    pub fn performers(&self, capability: &str) -> Result<(Vec<String>, Option<String>)> {
        self.store
            .performers(self.account.as_deref(), capability)
            .map_err(pimdir_error)
            .with_context(|| format!("Read who performs {capability}"))
    }

    /// Records `source` as the performer of `capability` for this account,
    /// or withdraws the choice when `None`, applied on the owner's next run.
    pub fn set_performer(&self, capability: &str, source: Option<&str>) -> Result<()> {
        let anchor = self
            .store
            .list_collections_by_account(self.account.as_deref())
            .map_err(pimdir_error)
            .context("List the account's collections")?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("The account holds no collection yet: run a sync first"))?;
        let action = PimdirAction::SetPerformer {
            capability: capability.to_owned(),
            source: source.map(String::from),
        };

        self.producer()?
            .enqueue(&anchor.id, &action, None)
            .map_err(pimdir_error)
            .context("Queue the choice of performer")?;
        Ok(())
    }
}

/// Builds the account context and the pimdir client the `pimdir` subcommand
/// runs against, the way each protocol-specific namespace builds its own.
pub fn build_pimdir_client(
    config: Config,
    name: String,
    mut account_config: AccountConfig,
) -> Result<(Account, PimdirClient)> {
    let pimdir_config = account_config
        .pimdir
        .take()
        .ok_or_else(|| anyhow!("pimdir config is missing for account `{name}`"))?;
    let account = Account {
        name: Some(name.to_string()),
        ..Account::from(config).merge(Account::from(account_config))
    };
    Ok((account, PimdirClient::new(pimdir_config)?))
}

/// The account this client reads, configured or derived.
///
/// A store synced by one account groups its collections under that name, or
/// under none in a store predating the grouping, so the common case needs no
/// configuration.
///
/// A store several accounts share is what `pimdir.account` answers, and
/// leaving it unset there errors rather than guess: picking one would show
/// the wrong mailbox set.
fn resolve_account(store: &PimdirReader, configured: Option<String>) -> Result<Option<String>> {
    if configured.is_some() {
        return Ok(configured);
    }

    let mut accounts: Vec<Option<String>> = store
        .list_collections()
        .map_err(pimdir_error)
        .context("List pimdir collections")?
        .into_iter()
        .map(|collection| collection.account)
        .collect();
    accounts.sort();
    accounts.dedup();

    match accounts.as_slice() {
        [] | [_] => Ok(accounts.into_iter().next().flatten()),
        _ => {
            let names: Vec<&str> = accounts
                .iter()
                .map(|account| account.as_deref().unwrap_or("<ungrouped>"))
                .collect();
            Err(anyhow!(
                "The pimdir store holds several accounts ({}); name one with `pimdir.account`",
                names.join(", "),
            ))
        }
    }
}
