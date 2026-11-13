//! # Flag arguments
//!
//! The flag and message-id arguments the `flag` and `message add`
//! commands take.

use std::fmt;

use clap::{Parser, ValueEnum};

/// A flag a unified command accepts.
///
/// The flags with one meaning across the backends declaring them, each a
/// capability of its own: `\Deleted`, the Maildir letters and the custom
/// keywords are reached through the protocol-specific commands instead.
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
    /// The message is marked important to its reader (RFC 8457).
    Important,
    /// The message is junk, spam.
    Junk,
}

impl fmt::Display for FlagArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Seen => "seen",
            Self::Answered => "answered",
            Self::Flagged => "flagged",
            Self::Draft => "draft",
            Self::Important => "important",
            Self::Junk => "junk",
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
            FlagArg::Important => MaildirFlag::Keyword("$Important".into()),
            FlagArg::Junk => MaildirFlag::Keyword("$Junk".into()),
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
            FlagArg::Important => IanaFlag::Important,
            FlagArg::Junk => IanaFlag::Junk,
        };

        Flag::from_iana(iana)
    }
}

/// Positional argument naming one or more messages.
#[derive(Debug, Parser)]
pub struct MessageIdsArg {
    /// The identifiers of the messages to act on.
    #[arg(name = "message_ids", value_name = "MESSAGE-IDS", required = true)]
    #[arg(num_args = 1..)]
    pub inner: Vec<String>,
}

impl MessageIdsArg {
    /// The ids, each once, in the order first given, so a repeated id is
    /// neither acted on nor counted twice.
    pub fn ids(&self) -> Vec<&str> {
        let mut ids: Vec<&str> = Vec::with_capacity(self.inner.len());
        for id in &self.inner {
            if !ids.contains(&id.as_str()) {
                ids.push(id);
            }
        }
        ids
    }
}

/// Repeatable option naming one or more flags.
#[derive(Debug, Parser)]
pub struct FlagsArg {
    /// The flags to apply, the option repeating for several.
    #[arg(name = "flags", value_name = "FLAG")]
    #[arg(long = "flag", short, required = true)]
    pub inner: Vec<FlagArg>,
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::MessageIdsArg;

    #[test]
    fn ids_are_required_and_a_repeated_one_counts_once() {
        assert!(MessageIdsArg::try_parse_from(["ids"]).is_err());

        let arg = MessageIdsArg::try_parse_from(["ids", "2", "1", "2"]).unwrap();
        assert_eq!(arg.ids(), ["2", "1"]);
    }
}
