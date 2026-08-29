//! # Gmail command
//!
//! The `gmail` command, dispatching onto its subcommand groups.

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    gmail::{
        attachments::GmailAttachmentsCommand, client::GmailClient, drafts::GmailDraftsCommand,
        history::GmailHistoryCommand, labels::GmailLabelsCommand, messages::GmailMessagesCommand,
        profile::GmailProfileCommand, settings::GmailSettingsCommand, threads::GmailThreadsCommand,
    },
};

/// Gmail-specific API.
///
/// Each subcommand group tracks one Gmail REST resource one to one.
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GmailCommand {
    #[command(subcommand)]
    Profile(GmailProfileCommand),
    #[command(subcommand)]
    #[command(alias = "label")]
    Labels(GmailLabelsCommand),
    #[command(subcommand)]
    #[command(visible_alias = "msg", alias = "message")]
    Messages(GmailMessagesCommand),
    #[command(subcommand)]
    #[command(alias = "attachment")]
    Attachments(GmailAttachmentsCommand),
    #[command(subcommand)]
    #[command(alias = "draft")]
    Drafts(GmailDraftsCommand),
    #[command(subcommand)]
    #[command(alias = "thread")]
    Threads(GmailThreadsCommand),
    #[command(subcommand)]
    History(GmailHistoryCommand),
    #[command(subcommand)]
    #[command(alias = "setting")]
    Settings(GmailSettingsCommand),
}

impl GmailCommand {
    /// Runs the subcommand against the account's Gmail client.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        match self {
            Self::Profile(cmd) => cmd.execute(printer, account, client),
            Self::Labels(cmd) => cmd.execute(printer, account, client),
            Self::Messages(cmd) => cmd.execute(printer, account, client),
            Self::Attachments(cmd) => cmd.execute(printer, account, client),
            Self::Drafts(cmd) => cmd.execute(printer, account, client),
            Self::Threads(cmd) => cmd.execute(printer, account, client),
            Self::History(cmd) => cmd.execute(printer, account, client),
            Self::Settings(cmd) => cmd.execute(printer, account, client),
        }
    }
}
