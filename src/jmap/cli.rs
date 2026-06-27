use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::account::context::Account;
use crate::jmap::{
    client::JmapClient, email::cli::JmapEmailCommand, identity::cli::JmapIdentityCommand,
    mailbox::cli::JmapMailboxCommand, query::JmapQueryCommand,
    submission::cli::JmapSubmissionCommand, thread::cli::JmapThreadCommand,
    vacation::cli::JmapVacationCommand,
};

/// JMAP-specific API.
///
/// Gives access to the raw JMAP API. Every CLI command matches the name of its
/// JMAP counterpart, grouped by domain: mailbox, email, thread, identity,
/// submission, vacation.
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum JmapCommand {
    Query(JmapQueryCommand),

    #[command(subcommand, visible_aliases = ["mbox"])]
    Mailbox(JmapMailboxCommand),
    #[command(subcommand)]
    Email(JmapEmailCommand),
    #[command(subcommand)]
    Thread(JmapThreadCommand),
    #[command(subcommand)]
    Identity(JmapIdentityCommand),
    #[command(subcommand)]
    Submission(JmapSubmissionCommand),
    #[command(subcommand, visible_alias = "vacation")]
    VacationResponse(JmapVacationCommand),
}

impl JmapCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut JmapClient,
    ) -> Result<()> {
        match self {
            Self::Mailbox(cmd) => cmd.execute(printer, account, client),
            Self::Email(cmd) => cmd.execute(printer, account, client),

            Self::Thread(cmd) => cmd.execute(printer, account, client),
            Self::Identity(cmd) => cmd.execute(printer, account, client),
            Self::Submission(cmd) => cmd.execute(printer, account, client),
            Self::VacationResponse(cmd) => cmd.execute(printer, account, client),
            Self::Query(cmd) => cmd.execute(printer, client),
        }
    }
}
