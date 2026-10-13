//! # pimdir command
//!
//! The `pimdir` command, dispatching onto its subcommands.

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    pimdir::{
        attachment::cli::PimdirAttachmentCommand, client::PimdirClient,
        mailbox::cli::PimdirMailboxCommand, message::cli::PimdirMessageCommand,
        performer::PimdirPerformerCommand, queue::cli::PimdirQueueCommand,
    },
};

/// pimdir-specific API.
///
/// Reaches what the store holds beside the mail a mailbox lists: the queue
/// of writes the sync engine has not applied yet, and the files the
/// account's messages attach.
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum PimdirCommand {
    #[command(subcommand)]
    Queue(PimdirQueueCommand),
    #[command(subcommand)]
    Message(PimdirMessageCommand),
    #[command(subcommand)]
    Mailbox(PimdirMailboxCommand),
    Performer(PimdirPerformerCommand),
    #[command(subcommand)]
    Attachment(PimdirAttachmentCommand),
}

impl PimdirCommand {
    /// Runs the subcommand against the account's pimdir store.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut PimdirClient,
    ) -> Result<()> {
        match self {
            Self::Queue(cmd) => cmd.execute(printer, account, client),
            Self::Message(cmd) => cmd.execute(printer, account, client),
            Self::Mailbox(cmd) => cmd.execute(printer, account, client),
            Self::Performer(cmd) => cmd.execute(printer, client),
            Self::Attachment(cmd) => cmd.execute(printer, account, client),
        }
    }
}
