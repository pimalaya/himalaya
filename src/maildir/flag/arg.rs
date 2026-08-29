//! # Maildir flag argument
//!
//! The argument naming one of the six standard Maildir flags.

use clap::ValueEnum;
use io_maildir::flag::MaildirFlag;

/// One of the six standard Maildir flags.
#[derive(Clone, Debug, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum FlagArg {
    /// `P`: the message has been resent or forwarded.
    Passed,
    /// `R`: the message has been replied to.
    Replied,
    /// `S`: the message has been read.
    Seen,
    /// `T`: the message is marked for deletion.
    Trashed,
    /// `D`: the message is an unsent draft.
    Draft,
    /// `F`: the message is marked for attention.
    Flagged,
}

impl From<FlagArg> for MaildirFlag {
    fn from(flag: FlagArg) -> Self {
        match flag {
            FlagArg::Passed => MaildirFlag::Passed,
            FlagArg::Replied => MaildirFlag::Replied,
            FlagArg::Seen => MaildirFlag::Seen,
            FlagArg::Trashed => MaildirFlag::Trashed,
            FlagArg::Draft => MaildirFlag::Draft,
            FlagArg::Flagged => MaildirFlag::Flagged,
        }
    }
}
