//! Himalaya wrapper around [`io_pimdir`]'s store and blob reader.

use std::path::PathBuf;

use anyhow::{Result, anyhow};
use io_pimdir::{PimdirBlobs, PimdirStore};

use crate::config::PimdirConfig;

/// Live pimdir client: an opened store (as one source) plus a connection-
/// independent blob reader over the same directory.
pub struct PimdirClient {
    pub(crate) store: PimdirStore,
    pub(crate) blobs: PimdirBlobs,
    /// The replica source this client opened the store as; a staged write is
    /// attributed to it.
    pub(crate) source: String,
}

impl PimdirClient {
    /// Opens (creating if absent) the pimdir store at the configured root.
    ///
    /// Reads are source-independent; the source only labels this client's writes.
    /// When `pimdir.source` is unset, it is **auto-detected**: a store synced as a
    /// single source (the local-sync case) has exactly one, so the app writes as
    /// it with no configuration; otherwise it falls back to `local`.
    pub fn new(config: PimdirConfig) -> Result<Self> {
        // Expand `~` and env vars: `root` is a `PathBuf` that carries the raw
        // `~/…` verbatim, and opening it unexpanded would silently create an empty
        // store at a literal `./~/…` relative to the cwd (an empty mailbox list),
        // not the intended home-relative one.
        let root = shellexpand::full(&config.root.to_string_lossy())
            .map(|expanded| PathBuf::from(expanded.into_owned()))
            .unwrap_or_else(|_| config.root.clone());

        let open = |source: &str| {
            PimdirStore::open(&root, source)
                .map_err(|err| anyhow!("Open pimdir store `{}`: {err}", root.display()))
        };

        let source = match config.source.clone() {
            Some(source) => source,
            None => {
                // Peek the store's sources (source-independent) to pick one.
                let probe = open("probe")?;
                match probe.distinct_sources()?.as_slice() {
                    [only] => only.clone(),
                    _ => "local".to_string(),
                }
            }
        };

        let store = open(&source)?;
        let blobs = PimdirBlobs::open(&root);
        Ok(Self {
            store,
            blobs,
            source,
        })
    }
}
