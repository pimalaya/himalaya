//! # mbox configuration
//!
//! The `mbox` block of an account.

use std::path::PathBuf;

use pimalaya_config::toml::{opt_shell_expanded_path, shell_expanded_path};
use serde::Deserialize;

/// mbox configuration: a directory of mbox files, plus the spool.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MboxConfig {
    /// The directory holding the mbox files, one file per mailbox,
    /// walked recursively.
    #[serde(deserialize_with = "shell_expanded_path")]
    pub root: PathBuf,
    /// The spool surfaced as the `INBOX` mailbox, typically `$MAIL`,
    /// wherever it lives.
    #[serde(default, deserialize_with = "opt_shell_expanded_path")]
    pub inbox: Option<PathBuf>,
    /// The variant new messages are written in, and whose quoting reads
    /// undo.
    #[serde(default)]
    pub format: MboxFormatConfig,
    /// Whether the children of mailbox `a` live in `a.sbd`, as
    /// Thunderbird lays them out, rather than in `a`.
    #[serde(default)]
    pub thunderbird: bool,
    /// The locks every write takes.
    #[serde(default)]
    pub lock: MboxLockConfig,
}

/// One of the four mbox variants.
///
/// Mirrors io-mbox's `MboxFormat`, kept local so the config schema does
/// not depend on any backend crate.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MboxFormatConfig {
    /// `From ` lines quoted with one `>`, lossy.
    Mboxo,
    /// Reversible quoting of `>*From ` lines.
    #[default]
    Mboxrd,
    /// mboxo quoting plus a `Content-Length` header.
    Mboxcl,
    /// No quoting, a `Content-Length` header delimits the body.
    Mboxcl2,
}

/// Per-account `mbox.lock.*` options, both locks taken by default.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MboxLockConfig {
    /// Whether to take the `<mbox>.lock` dotlock, which a spool
    /// directory the user cannot write to refuses.
    #[serde(default = "default_true")]
    pub dotlock: bool,
    /// Whether to take the fcntl lock.
    #[serde(default = "default_true")]
    pub fcntl: bool,
    /// Seconds to wait for a lock another process holds, 10 when
    /// unset.
    #[serde(default)]
    pub timeout: Option<u64>,
}

impl Default for MboxLockConfig {
    fn default() -> Self {
        Self {
            dotlock: true,
            fcntl: true,
            timeout: None,
        }
    }
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use crate::config::mbox::*;

    #[test]
    fn mbox_root_alone_takes_both_locks_and_writes_mboxrd() {
        let config: MboxConfig = toml::from_str(r#"root = "/tmp/mail""#).unwrap();

        assert_eq!(config.inbox, None);
        assert_eq!(config.format, MboxFormatConfig::Mboxrd);
        assert!(!config.thunderbird);
        assert_eq!(config.lock, MboxLockConfig::default());
        assert!(config.lock.dotlock && config.lock.fcntl);
    }

    #[test]
    fn mbox_options_parse() {
        let config: MboxConfig = toml::from_str(
            r#"
                root = "/tmp/mail"
                inbox = "/var/mail/me"
                format = "mboxcl2"
                thunderbird = true
                lock.dotlock = false
                lock.timeout = 30
            "#,
        )
        .unwrap();

        assert_eq!(config.inbox, Some(PathBuf::from("/var/mail/me")));
        assert_eq!(config.format, MboxFormatConfig::Mboxcl2);
        assert!(config.thunderbird);
        assert!(!config.lock.dotlock && config.lock.fcntl);
        assert_eq!(config.lock.timeout, Some(30));
    }
}
