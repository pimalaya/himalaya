//! # pimdir configuration
//!
//! The `pimdir` block of an account.

use std::path::PathBuf;

use pimalaya_config::toml::shell_expanded_path;
use serde::Deserialize;

/// pimdir configuration, a local store read as an offline cache.
///
/// The store is a SQLite index beside content-addressed blobs, populated
/// by the Neverest sync engine.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct PimdirConfig {
    /// The store directory, holding `pimdir.db` and `objects/`.
    #[serde(deserialize_with = "shell_expanded_path")]
    pub root: PathBuf,
    /// The sync engine account whose collections this client reads,
    /// per pimdir STORAGE §9.2.
    ///
    /// Usually left unset, a store synced by one account being read as
    /// that one. Set it for a store several accounts share, where guessing
    /// would show the wrong mailbox set.
    #[serde(default)]
    pub account: Option<String>,
}
