//! # JMAP command
//!
//! The `jmap` command, dispatching onto its subcommand groups.

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    jmap::{
        client::JmapClient, email::cli::JmapEmailCommand, identity::cli::JmapIdentityCommand,
        mailbox::cli::JmapMailboxCommand, query::JmapQueryCommand,
        submission::cli::JmapSubmissionCommand, thread::cli::JmapThreadCommand,
        vacation::cli::JmapVacationCommand,
    },
};

/// JMAP-specific API.
///
/// Each subcommand carries the name of its JMAP counterpart, grouped by
/// data type.
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum JmapCommand {
    /// Send a raw JMAP method-calls array and print the response.
    Query(JmapQueryCommand),
    /// Manage JMAP mailboxes.
    #[command(subcommand, visible_aliases = ["mbox"])]
    Mailbox(JmapMailboxCommand),
    /// Manage JMAP emails.
    #[command(subcommand)]
    Email(JmapEmailCommand),
    /// Manage JMAP threads.
    #[command(subcommand)]
    Thread(JmapThreadCommand),
    /// Manage JMAP sender identities.
    #[command(subcommand)]
    Identity(JmapIdentityCommand),
    /// Manage JMAP email submissions.
    #[command(subcommand)]
    Submission(JmapSubmissionCommand),
    /// Manage JMAP vacation response.
    #[command(subcommand, visible_alias = "vacation")]
    VacationResponse(JmapVacationCommand),
}

impl JmapCommand {
    /// Runs the subcommand against the account's JMAP client.
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
