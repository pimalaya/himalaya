//! # Maildir configuration
//!
//! The `maildir` block of an account.

use std::path::PathBuf;

use pimalaya_config::toml::shell_expanded_path;
use serde::Deserialize;

/// Maildir configuration.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MaildirConfig {
    /// The Maildir root, one directory per mailbox below it.
    #[serde(deserialize_with = "shell_expanded_path")]
    pub root: PathBuf,
    /// How custom, non-IANA keywords are read, if at all.
    #[serde(default)]
    pub keywords: MaildirKeywordsConfig,
    /// How the Maildirs of the store are laid out below the root.
    #[serde(default)]
    pub layout: MaildirLayoutConfig,
}

/// Per-account `maildir.keywords.*` options, both unset reading none.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MaildirKeywordsConfig {
    /// Whether keywords are resolved through each mailbox's own
    /// dovecot-keywords file, which maps a lowercase info-section letter
    /// to a keyword, default `false` leaving those letters unread.
    #[serde(default)]
    pub dovecot: bool,
    /// The body header keywords are read from, unset reading none.
    #[serde(default)]
    pub header: Option<MaildirKeywordHeaderConfig>,
}

/// Header carrying custom keywords inline with a message body.
///
/// Mirrors io-maildir's `KeywordHeader`, kept local so the config
/// schema does not depend on any backend crate.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum MaildirKeywordHeaderConfig {
    /// `X-Keywords`, comma-separated (OfflineIMAP, mbsync).
    XKeywords,
    /// `X-Label`, space-separated (mutt, notmuch).
    XLabel,
}

/// The layout of a Maildir store.
///
/// Mirrors io-maildir's `MaildirStore::maildirpp` switch, kept local so
/// the config schema does not depend on any backend crate.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq)]
pub enum MaildirLayoutConfig {
    /// One directory per Maildir, nested as the hierarchy is (`a/b`).
    #[default]
    #[serde(rename = "fs")]
    Fs,
    /// Maildir++, as Dovecot and Courier lay it out: the root is the
    /// inbox, and every other Maildir a dot-named sibling of its `cur`,
    /// `new` and `tmp` (`.a.b`).
    #[serde(rename = "maildir++")]
    MaildirPlusPlus,
}

#[cfg(test)]
mod tests {
    use crate::config::maildir::*;

    #[test]
    fn maildir_root_alone_keeps_keywords_off() {
        let config: MaildirConfig = toml::from_str(r#"root = "/tmp/mail""#).unwrap();
        assert!(!config.keywords.dovecot);
        assert_eq!(config.keywords.header, None);
    }

    #[test]
    fn maildir_keyword_options_parse() {
        let config: MaildirConfig = toml::from_str(
            r#"
                root = "/tmp/mail"
                keywords.dovecot = true
                keywords.header = "x-label"
            "#,
        )
        .unwrap();

        assert!(config.keywords.dovecot);
        assert_eq!(
            config.keywords.header,
            Some(MaildirKeywordHeaderConfig::XLabel)
        );
    }

    #[test]
    fn maildir_layout_defaults_to_fs_and_takes_maildir_plus_plus() {
        let config: MaildirConfig = toml::from_str(r#"root = "/tmp/mail""#).unwrap();
        assert_eq!(config.layout, MaildirLayoutConfig::Fs);

        let config: MaildirConfig = toml::from_str(
            r#"
                root = "/tmp/mail"
                layout = "maildir++"
            "#,
        )
        .unwrap();
        assert_eq!(config.layout, MaildirLayoutConfig::MaildirPlusPlus);

        let bad = toml::from_str::<MaildirConfig>("root = \"/m\"\nlayout = \"flat\"");
        assert!(bad.is_err());
    }
}
