//! Himalaya wrapper around [`io_pimdir`]'s store, blob reader and producer.
//!
//! The store belongs to the sync engine, not to Himalaya. Reads go through
//! [`PimdirReader`], the role that takes no lock (pimdir SPEC §8) and carries no
//! write at all, so a sync in flight neither blocks Himalaya nor is blocked by
//! it. Writes go through [`PimdirProducer`], which takes the shared lock for the
//! length of one enqueue: a producer stages actions for the owner to apply and
//! never mutates the index itself.
//!
//! The reader overlays the queue (pimdir SPEC §15.4), so an action this process
//! staged shows on the next read instead of waiting for the owner to apply it.

use std::path::PathBuf;

use anyhow::{Result, anyhow};
use io_pimdir::{PimdirBlobs, PimdirError, PimdirProducer, PimdirReader, PimdirStore};

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
    /// The store must exist: Himalaya reads a replica a sync populated, and
    /// creating one here would answer a mistyped root with an empty mailbox
    /// list instead of saying the path is wrong.
    pub fn new(config: PimdirConfig) -> Result<Self> {
        // Expand `~` and env vars: `root` is a `PathBuf` that carries the raw
        // `~/…` verbatim, and opening it unexpanded would look for a store at a
        // literal `./~/…` relative to the cwd.
        let root = shellexpand::full(&config.root.to_string_lossy())
            .map(|expanded| PathBuf::from(expanded.into_owned()))
            .unwrap_or_else(|_| config.root.clone());

        if !root.join("pimdir.db").exists() {
            return Err(anyhow!(
                "No pimdir store at `{}`; check `pimdir.root`, and run a sync to create one",
                root.display(),
            ));
        }

        // Reads overlay the queue (pimdir SPEC §15.4): what this client
        // staged is visible on the next read rather than on the next sync.
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

    /// Opens a producer for the length of one staging batch.
    ///
    /// A producer holds the store's shared lock, so it is opened for the writes
    /// it is about to stage and dropped as they land, rather than kept for the
    /// process's lifetime.
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
    /// Cancels one queued action, taking the store owner role for the length of
    /// the call (pimdir SPEC §15.5), and reports whether there was a row.
    ///
    /// Cancelling is an owner write and the only retraction a queued creation
    /// has. The role is entered here and released before this returns, so
    /// Himalaya never holds a handle that could drain the queue or collect the
    /// store. A sync in flight owns it, and that is refused at once rather than
    /// waited out: the action is still queued, and may have been applied by the
    /// time the user reads the message.
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
/// A store synced by one account groups its collections under that one name (or
/// under none, in a store written before the grouping), so the common case needs
/// no configuration. Several accounts in one store is what `pimdir.account`
/// answers, and leaving it unset there is an error rather than a guess: picking
/// one would silently show the wrong mailbox set.
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
