//! # Backend
//!
//! The `--backend` global flag: which backend the unified, cross-protocol
//! commands target, and the error of an account matching none.

use std::fmt;

use anyhow::{Error, anyhow};
use clap::ValueEnum;

use crate::{capability, config::AccountConfig};

/// Selects which backend a cross-protocol command targets.
///
/// `auto` picks the first configured backend the command supports, and a
/// named one pins it, which then bails when the account has no such block
/// or the operation no such arm. The protocol-specific subcommands ignore
/// it entirely.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum Backend {
    /// Let the command pick the first backend it is configured for.
    #[default]
    Auto,
    /// Pin the command to the account's IMAP backend.
    Imap,
    /// Pin the command to the account's JMAP backend.
    Jmap,
    /// Pin the command to the account's Gmail backend.
    Gmail,
    /// Pin the command to the account's Microsoft Graph backend.
    Msgraph,
    /// Pin the command to the account's Maildir backend.
    Maildir,
    /// Pin the command to the account's mbox backend.
    Mbox,
    /// Pin the command to the account's pimdir backend.
    Pimdir,
    /// Pin the command to the account's SMTP backend.
    Smtp,
    /// Pin the command to the account's ManageSieve backend.
    Sieve,
}

#[allow(unused)]
impl Backend {
    /// Every backend, compiled in or not, `auto` aside.
    pub const ALL: &[Self] = &[
        Self::Imap,
        Self::Jmap,
        Self::Gmail,
        Self::Msgraph,
        Self::Maildir,
        Self::Mbox,
        Self::Pimdir,
        Self::Smtp,
        Self::Sieve,
    ];

    /// Every backend this build compiles in, `auto` aside.
    pub const COMPILED: &[Self] = &[
        #[cfg(feature = "imap")]
        Self::Imap,
        #[cfg(feature = "jmap")]
        Self::Jmap,
        #[cfg(feature = "gmail")]
        Self::Gmail,
        #[cfg(feature = "msgraph")]
        Self::Msgraph,
        #[cfg(feature = "maildir")]
        Self::Maildir,
        #[cfg(feature = "mbox")]
        Self::Mbox,
        #[cfg(feature = "pimdir")]
        Self::Pimdir,
        #[cfg(feature = "smtp")]
        Self::Smtp,
        #[cfg(feature = "sieve")]
        Self::Sieve,
    ];

    /// Whether the IMAP arm of a unified command may run.
    pub fn allows_imap(self) -> bool {
        matches!(self, Self::Auto | Self::Imap)
    }

    /// Whether the JMAP arm of a unified command may run.
    pub fn allows_jmap(self) -> bool {
        matches!(self, Self::Auto | Self::Jmap)
    }

    /// Whether the Gmail arm of a unified command may run.
    pub fn allows_gmail(self) -> bool {
        matches!(self, Self::Auto | Self::Gmail)
    }

    /// Whether the Microsoft Graph arm of a unified command may run.
    pub fn allows_msgraph(self) -> bool {
        matches!(self, Self::Auto | Self::Msgraph)
    }

    /// Whether the Maildir arm of a unified command may run.
    pub fn allows_maildir(self) -> bool {
        matches!(self, Self::Auto | Self::Maildir)
    }

    /// Whether the mbox arm of a unified command may run.
    pub fn allows_mbox(self) -> bool {
        matches!(self, Self::Auto | Self::Mbox)
    }

    /// Whether the pimdir arm of a unified command may run.
    pub fn allows_pimdir(self) -> bool {
        matches!(self, Self::Auto | Self::Pimdir)
    }

    /// Whether the SMTP arm of a unified command may run.
    pub fn allows_smtp(self) -> bool {
        matches!(self, Self::Auto | Self::Smtp)
    }

    /// Whether the ManageSieve account-check arm may run.
    pub fn allows_sieve(self) -> bool {
        matches!(self, Self::Auto | Self::Sieve)
    }
}

impl fmt::Display for Backend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Auto => write!(f, "auto"),
            Self::Imap => write!(f, "imap"),
            Self::Jmap => write!(f, "jmap"),
            Self::Gmail => write!(f, "gmail"),
            Self::Msgraph => write!(f, "msgraph"),
            Self::Maildir => write!(f, "maildir"),
            Self::Mbox => write!(f, "mbox"),
            Self::Pimdir => write!(f, "pimdir"),
            Self::Smtp => write!(f, "smtp"),
            Self::Sieve => write!(f, "sieve"),
        }
    }
}

impl AccountConfig {
    /// The error of account `name` where nothing matches `backend`, the
    /// caller being able to use `supported` alone.
    ///
    /// Names the v1 `backend` table or the removed `m2dir` block when
    /// present, it being the likely cause. A `--backend` naming a block the
    /// account lacks is refused under `capability-missing`, as the resolver
    /// refuses a verb the filter leaves unserved.
    pub fn no_backend_error(&self, name: &str, backend: Backend, supported: &[Backend]) -> Error {
        let (reason, missing) = match backend {
            Backend::Auto => {
                let supported: Vec<String> = supported.iter().map(|b| format!("`{b}`")).collect();
                let supported = supported.join(", ");
                let reason =
                    format!("Account `{name}` configures no supported backend ({supported})");
                (reason, false)
            }
            backend if !supported.contains(&backend) => {
                let reason =
                    format!("Backend `{backend}` is not supported by this command or build");
                (reason, false)
            }
            backend => (format!("Account `{name}` has no `{backend}` block"), true),
        };

        let reason = if self.backend.is_some() {
            format!("{reason}: its v1 `backend` table is ignored since v2, see MIGRATION.md")
        } else if self.m2dir.is_some() {
            format!(
                "{reason}: its `m2dir` block is ignored since v3 removed m2dir, see MIGRATION.md"
            )
        } else {
            reason
        };

        if missing {
            capability::missing(reason)
        } else {
            anyhow!(reason)
        }
    }
}
