//! # Error codes
//!
//! A stable, machine-readable name for the failures a caller is expected
//! to act on, printed as `code` beside the message under `--json`.
//!
//! The message stays for humans and may be reworded at any time; the
//! code is the contract. A failure with no code prints as before, with
//! no `code` field. The mechanism (`CodedError`, `code_of`, the report)
//! lives in `pimalaya_cli::error`; only the vocabulary is Himalaya's.
//!
//! A pimdir store failure takes the code io-pimdir gives it
//! (`PimdirError::code`), spelled the same, so a caller reading every
//! pimdir app reads one vocabulary.

#[cfg(feature = "pimdir")]
use io_pimdir::client::PimdirError;
#[cfg(feature = "pimdir")]
use pimalaya_cli::error::CodedError;
use schemars::JsonSchema;
use serde::Serialize;

/// The stable codes, each serialised in kebab-case.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ErrorCode {
    /// The message is listed but its body is not local yet: a sync will
    /// bring it, so the caller waits rather than reports a loss.
    #[cfg(feature = "pimdir")]
    BodyPending,
    /// The message is nested too deep or holds too many parts or header
    /// lines to be read whole, so it is refused rather than read in
    /// part.
    MessageTooComplex,
    /// The pimdir store's schema is newer than Himalaya's: upgrade
    /// Himalaya.
    #[cfg(feature = "pimdir")]
    StoreNewer,
    /// The pimdir store has no schema yet: its sync engine creates it.
    #[cfg(feature = "pimdir")]
    StoreUncreated,
    /// The database at the pimdir root is not a pimdir store: another
    /// program wrote it.
    #[cfg(feature = "pimdir")]
    StoreForeign,
    /// The pimdir store's version records disagree.
    #[cfg(feature = "pimdir")]
    StoreCorrupt,
    /// The pimdir store is not at Himalaya's schema: run or upgrade its
    /// sync engine.
    #[cfg(feature = "pimdir")]
    StoreNotCurrent,
    /// The pimdir store was written by an earlier draft: run or upgrade
    /// its sync engine.
    #[cfg(feature = "pimdir")]
    StoreStale,
    /// The pimdir store names its bodies with a hash Himalaya does not
    /// compute.
    #[cfg(feature = "pimdir")]
    StoreHashAlgo,
    /// Another writer holds the pimdir store's write lock: retry.
    #[cfg(feature = "pimdir")]
    StoreBusy,
    /// A sync owns the pimdir store: retry once it ends.
    #[cfg(feature = "pimdir")]
    StoreOwned,
    /// A producer is staging a body in the pimdir store: retry.
    #[cfg(feature = "pimdir")]
    StoreStaging,
    /// A pimdir store verb is between two of its chunks: retry.
    #[cfg(feature = "pimdir")]
    StoreInFlight,
    /// The mailbox already holds the message, or has its creation
    /// queued: nothing was staged.
    #[cfg(feature = "pimdir")]
    ItemDuplicate,
    /// A source of the pimdir store does not support the write: nothing
    /// was staged.
    #[cfg(feature = "pimdir")]
    Unsupported,
    /// No source of the pimdir store can perform the intent.
    #[cfg(feature = "pimdir")]
    NoPerformer,
    /// Several sources of the pimdir store can perform the intent and
    /// none is chosen.
    #[cfg(feature = "pimdir")]
    AmbiguousPerformer,
    /// The pimdir store has no search index yet: its sync engine builds
    /// it.
    #[cfg(feature = "pimdir")]
    IndexMissing,
    /// The pimdir search index was built by another version: its sync
    /// engine rebuilds it.
    #[cfg(feature = "pimdir")]
    IndexOutdated,
    /// The SQLite Himalaya links lacks what the pimdir search index
    /// needs (FTS5, 3.43 or later).
    #[cfg(feature = "pimdir")]
    IndexFts5Missing,
    /// The pimdir search query is malformed or names an unknown field or
    /// value: nothing was searched.
    #[cfg(feature = "pimdir")]
    QueryRefused,
    /// The pimdir search index could not answer an accepted query, an
    /// implementation fault.
    #[cfg(feature = "pimdir")]
    QueryUnanswerable,
    /// The pimdir tag names no flag, or adds and removes one flag:
    /// nothing was staged.
    #[cfg(feature = "pimdir")]
    TagRefused,
}

/// The pimdir codes Himalaya relays, by io-pimdir's spelling, which
/// serialises each.
#[cfg(feature = "pimdir")]
const PIMDIR_CODES: &[(&str, ErrorCode)] = &[
    ("store-newer", ErrorCode::StoreNewer),
    ("store-uncreated", ErrorCode::StoreUncreated),
    ("store-foreign", ErrorCode::StoreForeign),
    ("store-corrupt", ErrorCode::StoreCorrupt),
    ("store-not-current", ErrorCode::StoreNotCurrent),
    ("store-stale", ErrorCode::StoreStale),
    ("store-hash-algo", ErrorCode::StoreHashAlgo),
    ("store-busy", ErrorCode::StoreBusy),
    ("store-owned", ErrorCode::StoreOwned),
    ("store-staging", ErrorCode::StoreStaging),
    ("store-in-flight", ErrorCode::StoreInFlight),
    ("item-duplicate", ErrorCode::ItemDuplicate),
    ("unsupported", ErrorCode::Unsupported),
    ("no-performer", ErrorCode::NoPerformer),
    ("ambiguous-performer", ErrorCode::AmbiguousPerformer),
    ("index-missing", ErrorCode::IndexMissing),
    ("index-outdated", ErrorCode::IndexOutdated),
    ("index-fts5-missing", ErrorCode::IndexFts5Missing),
    ("query-refused", ErrorCode::QueryRefused),
    ("query-unanswerable", ErrorCode::QueryUnanswerable),
    ("tag-refused", ErrorCode::TagRefused),
];

#[cfg(feature = "pimdir")]
impl ErrorCode {
    /// The code of a pimdir failure, `None` for one the caller cannot act
    /// on (an SQL or I/O failure, a malformed action).
    pub fn of_pimdir(err: &PimdirError) -> Option<Self> {
        let code = err.code();
        PIMDIR_CODES
            .iter()
            .find(|(pimdir, _)| *pimdir == code)
            .map(|(_, ours)| *ours)
    }
}

/// A pimdir failure as an error carrying its code, when it has one.
///
/// The message stays io-pimdir's: the caller adds its own context.
#[cfg(feature = "pimdir")]
pub fn pimdir_error(err: PimdirError) -> anyhow::Error {
    match ErrorCode::of_pimdir(&err) {
        Some(code) => CodedError::new(code, err.to_string()).into(),
        None => err.into(),
    }
}

#[cfg(all(test, feature = "pimdir"))]
mod tests {
    use super::*;

    #[test]
    fn a_relayed_pimdir_code_keeps_io_pimdirs_spelling() {
        for (pimdir, ours) in PIMDIR_CODES {
            assert_eq!(serde_json::to_value(ours).unwrap(), *pimdir);
        }
    }

    #[test]
    fn a_pimdir_failure_carries_its_code_under_context() {
        let err = pimdir_error(PimdirError::Busy).context("Stage add in `INBOX`");

        assert_eq!(
            pimalaya_cli::error::code_of::<ErrorCode>(&err),
            Some(ErrorCode::StoreBusy)
        );
        assert!(format!("{err:#}").contains("write lock"), "{err:#}");
    }
}
