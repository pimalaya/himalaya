use anyhow::Result;
use clap::Subcommand;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::jmap::{
    account::JmapAccount, email::command::JmapEmailCommand, identity::command::IdentityCommand,
    mailbox::command::JmapMailboxCommand, query::QueryCommand,
    submission::command::SubmissionCommand, thread::command::ThreadCommand,
    vacation::command::VacationCommand,
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
    Threads(ThreadCommand),
    #[command(subcommand)]
    #[command(aliases = ["identities"])]
    Identity(IdentityCommand),
    #[command(subcommand)]
    #[command(aliases = ["submissions", "submit"])]
    Submission(SubmissionCommand),
    #[command(subcommand)]
    #[command(alias = "vacation-response")]
    Vacation(VacationCommand),
    Query(QueryCommand),
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
