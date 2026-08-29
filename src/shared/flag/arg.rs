//! # Flag arguments
//!
//! The flag and message-id arguments the `flag` and `message add`
//! commands take.

use std::fmt;

use clap::{Parser, ValueEnum};

/// A flag a shared command accepts.
///
/// Strictly least-common-denominator: `\Deleted`, the Maildir letters and
/// the JMAP custom keywords are reached through the protocol-specific
/// commands instead.
#[derive(Clone, Debug, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum FlagArg {
    /// The message has been read.
    Seen,
    /// The message has been replied to.
    Answered,
    /// The message is marked for attention.
    Flagged,
    /// The message is an unsent draft.
    Draft,
}

impl fmt::Display for FlagArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Seen => "seen",
            Self::Answered => "answered",
            Self::Flagged => "flagged",
            Self::Draft => "draft",
        };
        f.write_str(name)
    }
}

#[cfg(feature = "maildir")]
impl From<&FlagArg> for io_maildir::flag::MaildirFlag {
    fn from(flag: &FlagArg) -> Self {
        use io_maildir::flag::MaildirFlag;

        match flag {
            FlagArg::Seen => MaildirFlag::Seen,
            FlagArg::Answered => MaildirFlag::Replied,
            FlagArg::Flagged => MaildirFlag::Flagged,
            FlagArg::Draft => MaildirFlag::Draft,
        }
    }
}

impl From<&FlagArg> for crate::email::flag::Flag {
    fn from(flag: &FlagArg) -> Self {
        use crate::email::flag::{Flag, IanaFlag};

        let iana = match flag {
            FlagArg::Seen => IanaFlag::Seen,
            FlagArg::Answered => IanaFlag::Answered,
            FlagArg::Flagged => IanaFlag::Flagged,
            FlagArg::Draft => IanaFlag::Draft,
        };

        Flag::from_iana(iana)
    }
}

/// Positional argument naming one or more messages.
#[derive(Debug, Parser)]
pub struct MessageIdsArg {
    /// The identifiers of the messages to act on.
    #[arg(name = "message_ids", value_name = "MESSAGE-IDS")]
    #[arg(num_args = 1..)]
    pub inner: Vec<String>,
}

/// Repeatable option naming one or more flags.
#[derive(Debug, Parser)]
pub struct FlagsArg {
    /// The flags to apply, the option repeating for several.
    #[arg(name = "flags", value_name = "FLAG")]
    #[arg(long = "flag", short, required = true)]
    pub inner: Vec<FlagArg>,
}
