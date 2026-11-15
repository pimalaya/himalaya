//! # Backend
//!
//! The `--backend` global flag: which backend the unified, cross-protocol
//! commands target, and the error of an account matching none.

use std::fmt;

use anyhow::{Error, anyhow};
use clap::ValueEnum;
use pimalaya_cli::error::CodedError;

use crate::{capability, config::AccountConfig, error::ErrorCode};

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

    /// The backend's name, as `--backend` and the lists spell it.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Imap => "imap",
            Self::Jmap => "jmap",
            Self::Gmail => "gmail",
            Self::Msgraph => "msgraph",
            Self::Maildir => "maildir",
            Self::Mbox => "mbox",
            Self::Pimdir => "pimdir",
            Self::Smtp => "smtp",
            Self::Sieve => "sieve",
        }
    }

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
        f.write_str(self.as_str())
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
        // NOTE: a block of a backend this build leaves out loads, so one
        // file serves every build; using it names the cargo feature.
        let unbuilt: Vec<Backend> = match backend {
            Backend::Auto => {
                let configured = self.configured();
                let usable = configured.iter().any(|b| supported.contains(b));
                configured
                    .into_iter()
                    .filter(|b| !usable && !Backend::COMPILED.contains(b))
                    .collect()
            }
            backend if !Backend::COMPILED.contains(&backend) => vec![backend],
            _ => Vec::new(),
        };

        if !unbuilt.is_empty() {
            return backend_not_built(&unbuilt);
        }

        let (reason, missing) = match backend {
            Backend::Auto => {
                let supported: Vec<String> = supported.iter().map(|b| format!("`{b}`")).collect();
                let supported = supported.join(", ");
                let reason =
                    format!("Account `{name}` configures no supported backend ({supported})");
                (reason, false)
            }
            backend if !supported.contains(&backend) => {
                let reason = format!("Backend `{backend}` is not supported by this command");
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

    /// The backends whose block the account holds, compiled in or not.
    pub(crate) fn configured(&self) -> Vec<Backend> {
        let blocks = [
            (Backend::Imap, self.imap.is_some()),
            (Backend::Jmap, self.jmap.is_some()),
            (Backend::Gmail, self.gmail.is_some()),
            (Backend::Msgraph, self.msgraph.is_some()),
            (Backend::Maildir, self.maildir.is_some()),
            (Backend::Mbox, self.mbox.is_some()),
            (Backend::Pimdir, self.pimdir.is_some()),
            (Backend::Smtp, self.smtp.is_some()),
            (Backend::Sieve, self.sieve.is_some()),
        ];

        blocks
            .into_iter()
            .filter_map(|(backend, present)| present.then_some(backend))
            .collect()
    }
}

/// The failure of a command needing `backends`, which this build leaves
/// out, coded `backend-not-built` and naming their cargo features.
fn backend_not_built(backends: &[Backend]) -> Error {
    let names: Vec<String> = backends.iter().map(|b| format!("`{b}`")).collect();
    let features: Vec<&str> = backends.iter().map(|b| b.as_str()).collect();
    let message = format!(
        "This build leaves out the {} backend, rebuild with `--features {}`",
        names.join(", "),
        features.join(","),
    );

    CodedError::new(ErrorCode::BackendNotBuilt, message).into()
}
