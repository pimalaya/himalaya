//! Merged runtime account — the DTO every command consumes.
//!
//! Built by the dispatch layer (`crate::cli`) in this order:
//!
//! 1. [`Account::default`] (all fields `None` / empty).
//! 2. Fold the global [`Config`] via `Account::from(config)`.
//! 3. Fold the selected `[accounts.<name>]` via [`Account::merge`]
//!    with `Account::from(account_config)`.
//!
//! Defaults are applied at consumption time by the `*` accessor
//! methods, not baked in during merge — keeping `Option<T>` fields
//! lets layers compose cleanly.

use std::{collections::HashMap, env::temp_dir, path::PathBuf};

use comfy_table::{presets, ContentArrangement};
use dirs::download_dir;

use crate::config::{AccountConfig, ComposerConfig, Config, ReaderConfig, TableArrangementConfig};

const DEFAULT_DATETIME_FMT: &str = "%F %R%:z";

#[derive(Clone, Debug, Default)]
pub struct Account {
    pub downloads_dir: Option<PathBuf>,
    pub table_preset: Option<String>,
    pub table_arrangement: Option<TableArrangementConfig>,

    pub datetime_fmt: Option<String>,
    pub datetime_local_tz: Option<bool>,

    /// User-defined composers. Only sourced from the global
    /// [`Config`]; account-level configs do not override these.
    pub composer: HashMap<String, ComposerConfig>,
    /// User-defined readers. See [`Account::composer`].
    pub reader: HashMap<String, ReaderConfig>,
}

impl Account {
    /// Folds `other`'s set fields on top of `self`. Each `Option`
    /// field is taken from `other` when `Some`, otherwise from
    /// `self`. The composer/reader maps are extended (entries from
    /// `other` overwrite same-named entries from `self`).
    pub fn merge(self, other: Self) -> Self {
        let mut composer = self.composer;
        composer.extend(other.composer);

        let mut reader = self.reader;
        reader.extend(other.reader);

        Self {
            downloads_dir: other.downloads_dir.or(self.downloads_dir),
            table_preset: other.table_preset.or(self.table_preset),
            table_arrangement: other.table_arrangement.or(self.table_arrangement),

            datetime_fmt: other.datetime_fmt.or(self.datetime_fmt),
            datetime_local_tz: other.datetime_local_tz.or(self.datetime_local_tz),

            composer,
            reader,
        }
    }

    /// Effective downloads directory. Tries the merged
    /// `downloads_dir` (shell-expanded), then the system default
    /// downloads dir, then the temp dir.
    pub fn downloads_dir(&self) -> PathBuf {
        self.downloads_dir
            .as_ref()
            .and_then(|dir| dir.to_str())
            .and_then(|dir| shellexpand::full(dir).ok())
            .map(|dir| PathBuf::from(dir.to_string()))
            .or_else(download_dir)
            .unwrap_or_else(temp_dir)
    }

    /// Effective `comfy_table` preset string. Defaults to
    /// `UTF8_FULL_CONDENSED`.
    pub fn table_preset(&self) -> &str {
        self.table_preset
            .as_deref()
            .unwrap_or(presets::UTF8_FULL_CONDENSED)
    }

    /// Effective `comfy_table` content arrangement. Defaults to
    /// `Dynamic`.
    pub fn table_arrangement(&self) -> ContentArrangement {
        self.table_arrangement
            .clone()
            .unwrap_or(TableArrangementConfig::Dynamic)
            .into()
    }

    /// Effective `chrono` `strftime` format for envelope DATE
    /// columns. Defaults to `%F %R%:z`.
    pub fn datetime_fmt(&self) -> &str {
        self.datetime_fmt.as_deref().unwrap_or(DEFAULT_DATETIME_FMT)
    }

    /// Whether to convert envelope `Date:` headers to the system
    /// local timezone. Defaults to `false`.
    pub fn datetime_local_tz(&self) -> bool {
        self.datetime_local_tz.unwrap_or(false)
    }
}

impl From<Config> for Account {
    fn from(config: Config) -> Self {
        Self {
            downloads_dir: config.downloads_dir,
            table_preset: config.table_preset,
            table_arrangement: config.table_arrangement,

            datetime_fmt: config.envelope.list.datetime_fmt,
            datetime_local_tz: config.envelope.list.datetime_local_tz,

            composer: config.message.composer,
            reader: config.message.reader,
        }
    }
}

impl From<AccountConfig> for Account {
    fn from(config: AccountConfig) -> Self {
        Self {
            downloads_dir: config.downloads_dir,
            table_preset: config.table_preset,
            table_arrangement: config.table_arrangement,

            datetime_fmt: config.envelope.list.datetime_fmt,
            datetime_local_tz: config.envelope.list.datetime_local_tz,

            composer: HashMap::new(),
            reader: HashMap::new(),
        }
    }
}
