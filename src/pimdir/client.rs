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

use anyhow::{Context, Result, anyhow};
use io_pimdir::{
    client::{
        PimdirError, PimdirStore,
        blobs::PimdirBlobs,
        producer::{PimdirActionStatus, PimdirEnqueue, PimdirEnqueued, PimdirProducer},
        reader::PimdirReader,
    },
    codec::{PimdirAction, PimdirActionHandle, PimdirStoreIdentity},
};
use pimalaya_cli::error::CodedError;

use crate::{
    account::context::Account,
    config::{AccountConfig, Config, pimdir::PimdirConfig},
    error::{ErrorCode, pimdir_error},
    pimdir::backend::MAIL_KIND,
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
            let message = format!(
                "No pimdir store at `{}`; check `pimdir.root`, and run a sync to create one",
                root.display(),
            );
            return Err(CodedError::new(ErrorCode::StoreUncreated, message).into());
        }

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

    /// Queues `actions` on `collection` as one batch (pimdir STORAGE
    /// §15.1), each gated once (§15.6), and answers their rows in order.
    ///
    /// A gate refusal or store failure stages nothing, the failure read by
    /// `context` with the refused id (`None` if none). No action, no write.
    pub(crate) fn enqueue_all(
        &self,
        collection: &str,
        actions: &[(&str, PimdirAction)],
        context: impl Fn(Option<&str>) -> String,
    ) -> Result<Vec<PimdirEnqueued>> {
        if actions.is_empty() {
            return Ok(Vec::new());
        }

        let mut producer = self.producer()?;
        let batch: Vec<PimdirEnqueue> = actions
            .iter()
            .map(|(_, action)| PimdirEnqueue::new(collection, action.clone()))
            .collect();

        producer.enqueue_batch(&batch).map_err(|err| {
            let refused = actions
                .iter()
                .find(|(_, action)| producer.check(collection, action).is_err())
                .map(|(id, _)| *id);
            pimdir_error(err).context(context(refused))
        })
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
    /// The store's identity, its id and incarnation (pimdir STORAGE §4.5),
    /// which a row's handle names.
    pub(crate) fn store_identity(&self) -> Result<PimdirStoreIdentity> {
        let cursor = self
            .store
            .change_cursor()
            .map_err(pimdir_error)
            .context("Read the pimdir store identity")?;

        Ok(PimdirStoreIdentity {
            store_id: cursor.store_id,
            incarnation: cursor.incarnation,
        })
    }

    /// The handle `given` names: as given when it names a store, else the
    /// bare id under this store's identity, as `pimdir queue list` prints.
    pub(crate) fn named(&self, given: &PimdirActionHandle) -> Result<PimdirActionHandle> {
        if given.store.is_some() {
            return Ok(given.clone());
        }

        Ok(PimdirActionHandle {
            id: given.id,
            store: Some(self.store_identity()?),
        })
    }
}

impl PimdirClient {
    /// Cancels one pending or parked row of the account's queue, returning
    /// its action kind.
    ///
    /// The owner role is held inside this call alone. Another account's
    /// row, a foreign-draft or damaged store and a sync in flight are
    /// refused at once; an unconfirmed row (§15.2) may have taken effect.
    pub fn cancel_queued(&self, handle: &PimdirActionHandle) -> Result<String> {
        let kind = match self.queue_row(handle)? {
            PimdirActionStatus::Pending { kind, .. }
            | PimdirActionStatus::Parked { kind, .. }
            | PimdirActionStatus::Unconfirmed { kind, .. } => kind,
            status => return Err(uncancellable(handle, &status)),
        };

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
            err => pimdir_error(err).context(format!("Cancel queued action {handle}")),
        })?;

        if !owner
            .drop_action(handle)
            .map_err(pimdir_error)
            .with_context(|| format!("Cancel queued action {handle}"))?
        {
            return Err(row_not_found(handle));
        }
        Ok(kind)
    }
}

impl PimdirClient {
    /// The source performing an intent `capability` (pimdir STORAGE
    /// §15.6), `None` when `collection`'s sources declare nothing.
    ///
    /// Several candidates and no recorded choice are the user's to settle:
    /// Himalaya never picks one.
    pub(crate) fn performer(
        &self,
        producer: &PimdirProducer,
        collection: &str,
        capability: &str,
    ) -> Result<Option<String>> {
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

    /// The one source syncing `collection` when its sources declare nothing
    /// (pimdir STORAGE §15.6, Annex B.2), ungated but named in the payload.
    ///
    /// None or several are refused rather than guessed.
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
    ///
    /// A source not performing `capability` is refused `no-performer`. The
    /// row anchors on the account's first mailbox, so `pimdir queue` reads
    /// it.
    pub fn set_performer(&self, capability: &str, source: Option<&str>) -> Result<()> {
        if let Some(source) = source {
            let (candidates, _) = self.performers(capability)?;
            if !candidates.iter().any(|candidate| candidate == source) {
                let message = match candidates.is_empty() {
                    true => format!("No source of this account performs {capability}"),
                    false => format!(
                        "{source} does not perform {capability} for this account: \
                         choose among {}",
                        candidates.join(", ")
                    ),
                };
                return Err(CodedError::new(ErrorCode::NoPerformer, message).into());
            }
        }

        let anchor = self
            .mail_collection_ids()?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("The account holds no mailbox yet: run a sync first"))?;
        let action = PimdirAction::SetPerformer {
            capability: capability.to_owned(),
            source: source.map(String::from),
        };

        self.producer()?
            .enqueue(&anchor, &action, None)
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
/// Several mail accounts and no `pimdir.account` errors as
/// `store-account-ambiguous` rather than guess; a name a store with
/// collections lacks errors as `store-account-unknown`, not read as empty.
fn resolve_account(store: &PimdirReader, configured: Option<String>) -> Result<Option<String>> {
    if let Some(account) = configured {
        let held = store
            .accounts_of(None)
            .map_err(pimdir_error)
            .context("List pimdir accounts")?;
        let named = Some(account.as_str());
        if !held.is_empty() && !held.iter().any(|held| held.as_deref() == named) {
            let message = format!(
                "The pimdir store holds no account `{account}`; check `pimdir.account`, \
                 or run a sync of that account"
            );
            return Err(CodedError::new(ErrorCode::StoreAccountUnknown, message).into());
        }
        return Ok(Some(account));
    }

    let accounts = store
        .accounts_of(Some(MAIL_KIND))
        .map_err(pimdir_error)
        .context("List pimdir accounts")?;

    match accounts.as_slice() {
        [] | [_] => Ok(accounts.into_iter().next().flatten()),
        _ => {
            let names: Vec<&str> = accounts
                .iter()
                .map(|account| account.as_deref().unwrap_or("<ungrouped>"))
                .collect();
            let message = format!(
                "The pimdir store holds several accounts of mail ({}); name one with \
                 `pimdir.account`",
                names.join(", "),
            );
            Err(CodedError::new(ErrorCode::StoreAccountAmbiguous, message).into())
        }
    }
}

/// The handle of the row `id` of the store `identity`, as `pimdir queue
/// list` and `pimdir status` print it.
pub(crate) fn handle(identity: &PimdirStoreIdentity, id: i64) -> String {
    PimdirActionHandle {
        id,
        store: Some(identity.clone()),
    }
    .to_string()
}

/// Why the row `handle` names, at `status`, cannot be cancelled: applied,
/// of another store (`store-reset`), or not queued (`row-not-found`).
pub fn uncancellable(handle: &PimdirActionHandle, status: &PimdirActionStatus) -> anyhow::Error {
    match status {
        PimdirActionStatus::Applied { .. } => anyhow!("Row {handle} was applied already"),
        PimdirActionStatus::Reset => store_reset(handle),
        _ => row_not_found(handle),
    }
}

/// The refusal of a handle another store answered, a copy restored since,
/// whose row id may name another row of this one.
pub(crate) fn store_reset(handle: &PimdirActionHandle) -> anyhow::Error {
    let message = format!(
        "Row {handle} was queued in another pimdir store than this one, a copy \
         restored since: nothing was done with the row"
    );
    CodedError::new(ErrorCode::StoreReset, message).into()
}

/// The refusal of a queue row the account's queue does not hold.
pub(crate) fn row_not_found(handle: &PimdirActionHandle) -> anyhow::Error {
    let message = format!(
        "No queued action with row {handle} in this account's queue; it may have been synced already"
    );
    CodedError::new(ErrorCode::RowNotFound, message).into()
}
