use std::fmt;

use clap::{Parser, ValueEnum};

/// Shared CLI flag argument for the cross-protocol `flags` and
/// `messages add` commands. The variant set is the strict
/// least-common-denominator across IMAP, JMAP and Maildir; backend
/// extras (`\Deleted`, Maildir `Trashed`/`Passed`, JMAP custom
/// keywords) live on the protocol-specific commands.
#[derive(Clone, Debug, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum FlagArg {
    Seen,
    Answered,
    Flagged,
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

#[cfg(feature = "imap")]
impl FlagArg {
    pub fn imap(&self) -> io_imap::types::flag::Flag<'static> {
        use io_imap::types::flag::Flag;

        match self {
            Self::Seen => Flag::Seen,
            Self::Answered => Flag::Answered,
            Self::Flagged => Flag::Flagged,
            Self::Draft => Flag::Draft,
        }
    }
}

#[cfg(feature = "jmap")]
impl FlagArg {
    pub fn jmap(&self) -> &'static str {
        match self {
            Self::Seen => "$seen",
            Self::Answered => "$answered",
            Self::Flagged => "$flagged",
            Self::Draft => "$draft",
        }
    }
}

#[cfg(feature = "maildir")]
impl From<&FlagArg> for io_maildir::flag::types::MaildirFlag {
    fn from(flag: &FlagArg) -> Self {
        use io_maildir::flag::types::MaildirFlag;

        match flag {
            FlagArg::Seen => MaildirFlag::Seen,
            FlagArg::Answered => MaildirFlag::Replied,
            FlagArg::Flagged => MaildirFlag::Flagged,
            FlagArg::Draft => MaildirFlag::Draft,
        }
    }
}

impl From<&FlagArg> for io_email::flag::types::Flag {
    fn from(flag: &FlagArg) -> Self {
        use io_email::flag::types::{Flag, IanaFlag};

        let iana = match flag {
            FlagArg::Seen => IanaFlag::Seen,
            FlagArg::Answered => IanaFlag::Answered,
            FlagArg::Flagged => IanaFlag::Flagged,
            FlagArg::Draft => IanaFlag::Draft,
        };

        Flag::from_iana(iana)
    }
}

/// Positional argument holding one or more message identifiers.
#[derive(Debug, Parser)]
pub struct MessageIdsArg {
    /// Message Identifier(s).
    #[arg(name = "message_ids", value_name = "MESSAGE-IDS")]
    #[arg(num_args = 1..)]
    pub inner: Vec<String>,
}

/// Repeatable option holding one or more flags to apply to messages.
#[derive(Debug, Parser)]
pub struct FlagsArg {
    /// Flag(s) to apply. Repeat the option to pass several (e.g. `-f
    /// seen -f flagged`).
    #[arg(name = "flags", value_name = "FLAG")]
    #[arg(long = "flag", short, required = true)]
    pub inner: Vec<FlagArg>,
}
