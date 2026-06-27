use clap::Parser;

const INBOX: &str = "INBOX";

/// The optional mailbox name argument parser.
#[derive(Debug, Parser)]
pub struct MailboxNameOptionalArg {
    /// The name of the mailbox.
    #[arg(name = "mailbox_name", value_name = "MAILBOX", default_value = INBOX)]
    pub inner: String,
}

impl Default for MailboxNameOptionalArg {
    fn default() -> Self {
        Self {
            inner: INBOX.into(),
        }
    }
}

/// The optional mailbox name flag parser.
#[derive(Debug, Parser)]
pub struct MailboxNameOptionalFlag {
    /// The name of the mailbox.
    #[arg(long = "mailbox", short = 'm')]
    #[arg(name = "mailbox_name", value_name = "NAME", default_value = INBOX)]
    pub inner: String,
}

impl Default for MailboxNameOptionalFlag {
    fn default() -> Self {
        Self {
            inner: INBOX.into(),
        }
    }
}

/// The no-select mailbox flag parser.
#[derive(Debug, Parser)]
pub struct MailboxNoSelectFlag {
    /// Do not select the given mailbox before performing the current
    /// action.
    ///
    /// This argument is useful when stateful IMAP sessions are used,
    /// for example with Sirup CLI:
    ///
    /// https://github.com/pimalaya/sirup
    #[arg(long = "no-select", default_value_t)]
    #[arg(name = "mailbox_no_select")]
    pub inner: bool,
}

/// The required mailbox name argument parser.
#[derive(Debug, Parser)]
pub struct MailboxNameArg {
    /// The name of the mailbox.
    #[arg(name = "mailbox_name", value_name = "MAILBOX")]
    pub inner: String,
}

/// The target mailbox name argument parser.
#[derive(Debug, Clone, Parser)]
pub struct TargetMailboxNameArg {
    /// The name of the target mailbox.
    #[arg(name = "target_mailbox_name", value_name = "TARGET")]
    pub inner: String,
}
