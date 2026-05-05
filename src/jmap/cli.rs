use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::jmap::{
    account::JmapAccount, email::cli::JmapEmailCommand, identity::cli::JmapIdentityCommand,
    mailbox::cli::JmapMailboxCommand, query::JmapQueryCommand,
    submission::cli::JmapSubmissionCommand, thread::cli::JmapThreadCommand,
    vacation::cli::JmapVacationCommand,
};

/// JMAP CLI (requires the `jmap` cargo feature).
///
/// This command gives you access to the JMAP CLI API, and allows you
/// to manage JMAP mailboxes, threads, emails, identities, submissions
/// and vacation responses.
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum JmapCommand {
    #[command(subcommand)]
    #[command(visible_aliases = ["mbox"])]
    Mailboxes(JmapMailboxCommand),

    #[command(subcommand)]
    #[command(visible_aliases = ["msg"])]
    Emails(JmapEmailCommand),

    #[command(subcommand)]
    Threads(JmapThreadCommand),
    #[command(subcommand)]
    #[command(aliases = ["identities"])]
    Identity(JmapIdentityCommand),
    #[command(subcommand)]
    #[command(aliases = ["submissions", "submit"])]
    Submission(JmapSubmissionCommand),
    #[command(subcommand)]
    #[command(alias = "vacation-response")]
    Vacation(JmapVacationCommand),
    Query(JmapQueryCommand),
}

impl JmapCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        match self {
            Self::Mailboxes(cmd) => cmd.execute(printer, account),
            Self::Emails(cmd) => cmd.execute(printer, account),

            Self::Threads(cmd) => cmd.execute(printer, account),
            Self::Identity(cmd) => cmd.execute(printer, account),
            Self::Submission(cmd) => cmd.execute(printer, account),
            Self::Vacation(cmd) => cmd.execute(printer, account),
            Self::Query(cmd) => cmd.execute(printer, account),
        }
    }
}
