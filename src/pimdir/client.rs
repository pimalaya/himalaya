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

use anyhow::{Result, anyhow};
use io_pimdir::{
    client::{
        PimdirError, PimdirStore, blobs::PimdirBlobs, producer::PimdirProducer,
        reader::PimdirReader,
    },
    codec::PimdirAction,
    object::PimdirObject,
};
use log::warn;

use crate::{
    account::context::Account,
    config::{AccountConfig, Config, PimdirConfig},
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
    /// The account grouping this client's collections (pimdir SPEC §9.2), or
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
            .map_err(|err| anyhow!("Open pimdir store `{}`: {err}", root.display()))?
            .with_pending();

        let account = resolve_account(&store, config.account.clone())?;
        let blobs = PimdirBlobs::open(&root, store.hash_algo());

        Ok(Self {
            store,
            blobs,
            root,
            account,
        })
    }

    /// Queues `action` through `producer`, warning about each capability
    /// its source supports in part (pimdir STORAGE §15.6).
    pub(crate) fn enqueue(
        &self,
        producer: &mut PimdirProducer,
        collection: &str,
        action: &PimdirAction,
        object: Option<&PimdirObject>,
    ) -> Result<i64, PimdirError> {
        let partials = producer.check(collection, action)?;
        let id = producer.enqueue(collection, action, object)?;
        for partial in &partials {
            warn!("{partial}");
        }
        Ok(id)
    }

    /// Opens a producer for the length of one staging batch.
    ///
    /// A producer holds the store's shared lock, so it is opened for the
    /// writes it stages and dropped as they land.
    pub(crate) fn producer(&self) -> Result<PimdirProducer> {
        let producer = PimdirProducer::open(&self.root, PRODUCER)
            .map_err(|err| anyhow!("Open pimdir producer `{}`: {err}", self.root.display()))?;

        Ok(match &self.account {
            Some(account) => producer.for_account(account),
            None => producer,
        })
    }
}

impl PimdirClient {
    /// Cancels one queued action, reporting whether there was a row.
    ///
    /// Cancelling is an owner write, and the only retraction a queued
    /// creation has. The role is entered and released inside this call, so
    /// Himalaya never holds a handle that could drain the queue or collect
    /// the store.
    ///
    /// A sync in flight owns the store, and that is refused at once rather
    /// than waited out: the action is still queued, and may have been applied
    /// by the time the user reads the message.
    pub fn cancel_queued(&self, id: i64) -> Result<bool> {
        PimdirStore::cancel_action(&self.root, id).map_err(|err| match err {
            PimdirError::Owned(_) => anyhow!(
                "A sync is running on `{}`, so the queue cannot be edited; \
                 the action may have been applied already",
                self.root.display(),
            ),
            err => anyhow!("Cancel queued action {id}: {err}"),
        })
    }
}

impl PimdirClient {
    /// The source performing an intent `capability` for this account
    /// (pimdir STORAGE §15.6), `None` in a store whose sources declare
    /// nothing, where the owner picks as it always did.
    ///
    /// Several candidates and no recorded choice is the user's to settle:
    /// Himalaya never picks one.
    pub(crate) fn performer(
        &self,
        producer: &PimdirProducer,
        collection: &str,
        capability: &str,
    ) -> Result<Option<String>> {
        let declared = producer
            .capabilities(collection)
            .map_err(|err| anyhow!("Read the capabilities of `{collection}`: {err}"))?
            .iter()
            .any(|source| source.declared.is_some());
        if !declared {
            return Ok(None);
        }

        match producer.performer(collection, capability, None) {
            Ok(source) => Ok(Some(source)),
            Err(PimdirError::Ambiguous { candidates, .. }) => Err(anyhow!(
                "Several sources can perform {capability} for this account ({}): \
                 choose one with `himalaya pimdir performer {capability} <SOURCE>`",
                candidates.join(", "),
            )),
            Err(err) => Err(anyhow!("{err}")),
        }
    }

    /// The sources able to perform `capability` for this account, and the
    /// one the user chose among them, if any.
    pub fn performers(&self, capability: &str) -> Result<(Vec<String>, Option<String>)> {
        self.store
            .performers(self.account.as_deref(), capability)
            .map_err(|err| anyhow!("Read who performs {capability}: {err}"))
    }

    /// Records `source` as the performer of `capability` for this account,
    /// or withdraws the choice when `None`, applied on the owner's next run.
    pub fn set_performer(&self, capability: &str, source: Option<&str>) -> Result<()> {
        let anchor = self
            .store
            .list_collections_by_account(self.account.as_deref())
            .map_err(|err| anyhow!("List the account's collections: {err}"))?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("The account holds no collection yet: run a sync first"))?;
        let action = PimdirAction::SetPerformer {
            capability: capability.to_owned(),
            source: source.map(String::from),
        };

        self.producer()?
            .enqueue(&anchor.id, &action, None)
            .map_err(|err| anyhow!("Queue the choice of performer: {err}"))?;
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
    let account = Account::from(config).merge(Account::from(account_config));
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
        .map_err(|err| anyhow!("List pimdir collections: {err}"))?
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
