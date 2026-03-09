use clap::Parser;

const INBOX: &str = "INBOX";

/// The optional mailbox name flag parser.
#[derive(Debug, Parser)]
pub struct MailboxNameOptionalFlag {
    /// The name of the mailbox.
    #[arg(long = "mailbox", short = 'm')]
    #[arg(name = "mailbox_name", value_name = "NAME", default_value = INBOX)]
    pub name: String,
}

impl Default for MailboxNameOptionalFlag {
    fn default() -> Self {
        Self {
            name: INBOX.to_owned(),
        }
    }
}

/// The optional mailbox name argument parser.
#[derive(Debug, Parser)]
pub struct MailboxNameOptionalArg {
    /// The name of the mailbox.
    #[arg(name = "mailbox_name", value_name = "MAILBOX", default_value = INBOX)]
    pub name: String,
}

impl Default for MailboxNameOptionalArg {
    fn default() -> Self {
        Self {
            name: INBOX.to_owned(),
        }
    }
}

/// The required mailbox name argument parser.
#[derive(Debug, Parser)]
pub struct MailboxNameArg {
    /// The name of the mailbox.
    #[arg(name = "mailbox_name", value_name = "MAILBOX")]
    pub name: String,
}

/// The optional source mailbox name flag parser.
#[derive(Debug, Parser)]
pub struct SourceMailboxNameOptionalFlag {
    /// The name of the source mailbox.
    #[arg(long = "mailbox", short = 'm')]
    #[arg(name = "source_mailbox_name", value_name = "SOURCE", default_value = INBOX)]
    pub name: String,
}

/// The target mailbox name argument parser.
#[derive(Debug, Parser)]
pub struct TargetMailboxNameArg {
    /// The name of the target mailbox.
    #[arg(name = "target_mailbox_name", value_name = "TARGET")]
    pub name: String,
}
