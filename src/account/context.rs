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
const DEFAULT_MAILBOX_ALIAS: &str = "inbox";
const DEFAULT_ENVELOPES_LIST_PAGE_SIZE: u32 = 25;

#[derive(Clone, Debug, Default)]
pub struct Account {
    pub downloads_dir: Option<PathBuf>,
    pub table_preset: Option<String>,
    pub table_arrangement: Option<TableArrangementConfig>,

    pub datetime_fmt: Option<String>,
    pub datetime_local_tz: Option<bool>,
    pub envelopes_list_page_size: Option<u32>,

    /// Mailbox aliases, keys lowercased. Populated from
    /// `mailbox.alias` at the global and account levels; account
    /// entries overwrite same-named global entries.
    pub mailbox_alias: HashMap<String, String>,

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
        let mut mailbox_alias = self.mailbox_alias;
        mailbox_alias.extend(other.mailbox_alias);

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
            envelopes_list_page_size: other
                .envelopes_list_page_size
                .or(self.envelopes_list_page_size),

            mailbox_alias,

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

    /// Effective default page size for `envelopes list` when the
    /// `-s/--page-size` flag is not passed. Defaults to 25.
    pub fn envelopes_list_page_size(&self) -> u32 {
        self.envelopes_list_page_size
            .unwrap_or(DEFAULT_ENVELOPES_LIST_PAGE_SIZE)
    }

    /// Resolves `name` through the alias map.
    ///
    /// Lookup is case-insensitive on the alias name. When `name`
    /// matches an alias, the stored id is returned verbatim;
    /// otherwise `name` itself is returned, allowing callers to pass
    /// either an alias or a raw backend id transparently.
    pub fn resolve_mailbox<'a>(&'a self, name: &'a str) -> &'a str {
        let key = name.to_lowercase();
        self.mailbox_alias
            .get(&key)
            .map(String::as_str)
            .unwrap_or(name)
    }

    /// Resolved id of the implicit default mailbox.
    ///
    /// Returns the id mapped to the `inbox` alias (case-insensitive),
    /// or `None` when no such alias is configured. Used by shared
    /// commands when `-m/--mailbox` is omitted.
    pub fn default_mailbox(&self) -> Option<&str> {
        self.mailbox_alias
            .get(DEFAULT_MAILBOX_ALIAS)
            .map(String::as_str)
    }
}

/// Lowercases every key of `aliases`, leaving values untouched. Used at
/// the [`Config`] / [`AccountConfig`] -> [`Account`] boundary so that
/// the merge and the [`Account::resolve_mailbox`] lookup can both rely
/// on already-normalized keys.
fn lowercase_alias_keys(aliases: HashMap<String, String>) -> HashMap<String, String> {
    aliases
        .into_iter()
        .map(|(k, v)| (k.to_lowercase(), v))
        .collect()
}

impl From<Config> for Account {
    fn from(config: Config) -> Self {
        Self {
            downloads_dir: config.downloads_dir,
            table_preset: config.table_preset,
            table_arrangement: config.table_arrangement,

            datetime_fmt: config.envelope.list.datetime_fmt,
            datetime_local_tz: config.envelope.list.datetime_local_tz,
            envelopes_list_page_size: config.envelope.list.page_size,

            mailbox_alias: lowercase_alias_keys(config.mailbox.alias),

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
            envelopes_list_page_size: config.envelope.list.page_size,

            mailbox_alias: lowercase_alias_keys(config.mailbox.alias),

            composer: HashMap::new(),
            reader: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MailboxConfig;

    fn account_with_aliases(pairs: &[(&str, &str)]) -> Account {
        let alias = pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        let config = Config {
            mailbox: MailboxConfig { alias },
            ..Config::default()
        };
        Account::from(config)
    }

    #[test]
    fn resolve_mailbox_returns_alias_target() {
        let account = account_with_aliases(&[("inbox", "INBOX")]);
        assert_eq!(account.resolve_mailbox("inbox"), "INBOX");
    }

    #[test]
    fn resolve_mailbox_lookup_is_case_insensitive() {
        let account = account_with_aliases(&[("inbox", "INBOX")]);
        assert_eq!(account.resolve_mailbox("INBOX"), "INBOX");
        assert_eq!(account.resolve_mailbox("Inbox"), "INBOX");
        assert_eq!(account.resolve_mailbox("iNbOx"), "INBOX");
    }

    #[test]
    fn resolve_mailbox_normalizes_keys_stored_with_any_case() {
        let account = account_with_aliases(&[("INBOX", "raw-id")]);
        assert_eq!(account.resolve_mailbox("inbox"), "raw-id");
    }

    #[test]
    fn resolve_mailbox_preserves_id_case() {
        let account = account_with_aliases(&[("sent", "Sent Items")]);
        assert_eq!(account.resolve_mailbox("SENT"), "Sent Items");
    }

    #[test]
    fn resolve_mailbox_falls_back_to_input_when_no_alias() {
        let account = account_with_aliases(&[]);
        assert_eq!(account.resolve_mailbox("INBOX"), "INBOX");
    }

    #[test]
    fn default_mailbox_returns_inbox_alias() {
        let account = account_with_aliases(&[("inbox", "raw-id")]);
        assert_eq!(account.default_mailbox(), Some("raw-id"));
    }

    #[test]
    fn default_mailbox_is_none_without_inbox_alias() {
        let account = account_with_aliases(&[("sent", "Sent Items")]);
        assert_eq!(account.default_mailbox(), None);
    }

    #[test]
    fn merge_lets_account_override_global_alias() {
        let global = account_with_aliases(&[("inbox", "INBOX"), ("sent", "Sent")]);
        let per_account = account_with_aliases(&[("inbox", "Mailbox/0")]);
        let merged = global.merge(per_account);
        assert_eq!(merged.resolve_mailbox("inbox"), "Mailbox/0");
        assert_eq!(merged.resolve_mailbox("sent"), "Sent");
    }
}
