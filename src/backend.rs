//! # Backend
//!
//! The `--backend` global flag: which backend the shared, cross-protocol
//! commands target.

use std::fmt;

use clap::ValueEnum;

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
    /// Pin the command to the account's m2dir backend.
    M2dir,
    /// Pin the command to the account's pimdir backend.
    Pimdir,
    /// Pin the command to the account's SMTP backend.
    Smtp,
    /// Pin the command to the account's ManageSieve backend.
    Sieve,
}

#[allow(unused)]
impl Backend {
    /// Whether the IMAP arm of a shared command may run.
    pub fn allows_imap(self) -> bool {
        matches!(self, Self::Auto | Self::Imap)
    }

    /// Whether the JMAP arm of a shared command may run.
    pub fn allows_jmap(self) -> bool {
        matches!(self, Self::Auto | Self::Jmap)
    }

    /// Whether the Gmail arm of a shared command may run.
    pub fn allows_gmail(self) -> bool {
        matches!(self, Self::Auto | Self::Gmail)
    }

    /// Whether the Microsoft Graph arm of a shared command may run.
    pub fn allows_msgraph(self) -> bool {
        matches!(self, Self::Auto | Self::Msgraph)
    }

    /// Whether the Maildir arm of a shared command may run.
    pub fn allows_maildir(self) -> bool {
        matches!(self, Self::Auto | Self::Maildir)
    }

    /// Whether the m2dir arm of a shared command may run.
    pub fn allows_m2dir(self) -> bool {
        matches!(self, Self::Auto | Self::M2dir)
    }

    /// Whether the pimdir arm of a shared command may run.
    pub fn allows_pimdir(self) -> bool {
        matches!(self, Self::Auto | Self::Pimdir)
    }

    /// Whether the SMTP arm of a shared command may run.
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
            Self::M2dir => write!(f, "m2dir"),
            Self::Pimdir => write!(f, "pimdir"),
            Self::Smtp => write!(f, "smtp"),
            Self::Sieve => write!(f, "sieve"),
        }
    }
}
