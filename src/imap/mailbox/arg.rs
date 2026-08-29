//! # IMAP mailbox arguments
//!
//! The mailbox name arguments and flags the IMAP commands share.

use clap::Parser;

/// The mailbox an argument defaults to.
const INBOX: &str = "INBOX";

/// Optional positional naming a mailbox, `INBOX` by default.
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

/// Optional flag naming a mailbox, `INBOX` by default.
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

/// Flag skipping the `SELECT` a command would otherwise issue.
#[derive(Debug, Parser)]
pub struct MailboxNoSelectFlag {
    /// Do not select the mailbox before acting on it.
    ///
    /// Useful over a stateful IMAP session, a Sirup proxy for instance,
    /// where the mailbox is already selected.
    #[arg(long = "no-select", default_value_t)]
    #[arg(name = "mailbox_no_select")]
    pub inner: bool,
}

/// Required positional naming a mailbox.
#[derive(Debug, Parser)]
pub struct MailboxNameArg {
    /// The name of the mailbox.
    #[arg(name = "mailbox_name", value_name = "MAILBOX")]
    pub inner: String,
}

/// Required positional naming the mailbox an operation targets.
#[derive(Debug, Clone, Parser)]
pub struct TargetMailboxNameArg {
    /// The name of the target mailbox.
    #[arg(name = "target_mailbox_name", value_name = "TARGET")]
    pub inner: String,
}
