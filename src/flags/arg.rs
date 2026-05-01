use clap::{Parser, ValueEnum};

#[derive(Clone, Debug, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum FlagArg {
    Seen,
    Answered,
    Flagged,
    Deleted,
    Draft,
}

#[cfg(feature = "imap")]
impl FlagArg {
    pub fn imap(&self) -> io_imap::types::flag::Flag<'static> {
        use io_imap::types::flag::Flag;

        match self {
            Self::Seen => Flag::Seen,
            Self::Answered => Flag::Answered,
            Self::Flagged => Flag::Flagged,
            Self::Deleted => Flag::Deleted,
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
            Self::Deleted => "$deleted",
            Self::Draft => "$draft",
        }
    }
}

#[cfg(feature = "maildir")]
impl From<&FlagArg> for io_maildir::flag::Flag {
    fn from(flag: &FlagArg) -> Self {
        use io_maildir::flag::Flag;

        match flag {
            FlagArg::Seen => Flag::Seen,
            FlagArg::Answered => Flag::Replied,
            FlagArg::Flagged => Flag::Flagged,
            FlagArg::Deleted => Flag::Trashed,
            FlagArg::Draft => Flag::Draft,
        }
    }
}

#[derive(Debug, Parser)]
pub struct MessageIdsArg {
    /// Identifier(s) of message(s) (IMAP UID, JMAP email ID, Maildir filename id).
    #[arg(name = "message_ids", value_name = "ID")]
    #[arg(num_args = 1..)]
    pub inner: Vec<String>,
}

#[derive(Debug, Parser)]
pub struct FlagsArg {
    /// Flag(s) to apply.
    #[arg(long = "flag", short, required = true, num_args = 1..)]
    pub inner: Vec<FlagArg>,
}

#[derive(Debug, Parser)]
pub struct MailboxFlag {
    /// Mailbox name or path (IMAP mailbox / Maildir path).
    #[arg(
        long = "mailbox",
        short = 'm',
        value_name = "NAME",
        default_value = "Inbox"
    )]
    pub inner: String,
}
