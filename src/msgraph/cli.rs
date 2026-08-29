//! # Microsoft Graph command
//!
//! The `msgraph` command, dispatching onto its subcommand groups.

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    msgraph::{
        attachments::cli::MsgraphAttachmentsCommand, client::MsgraphClient,
        mail_folders::cli::MsgraphMailFoldersCommand, messages::cli::MsgraphMessagesCommand,
        profile::cli::MsgraphProfileCommand,
    },
};

/// Microsoft Graph-specific API.
///
/// Each subcommand group tracks one Graph mail resource one to one.
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum MsgraphCommand {
    #[command(subcommand)]
    Profile(MsgraphProfileCommand),
    #[command(subcommand, visible_alias = "folders", aliases = ["mail-folder", "folder"])]
    MailFolders(MsgraphMailFoldersCommand),
    #[command(subcommand, visible_alias = "msg", alias = "message")]
    Messages(MsgraphMessagesCommand),
    #[command(subcommand, alias = "attachment")]
    Attachments(MsgraphAttachmentsCommand),
}

impl MsgraphCommand {
    /// Runs the subcommand against the account's Graph client.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut MsgraphClient,
    ) -> Result<()> {
        match self {
            Self::Profile(cmd) => cmd.execute(printer, account, client),
            Self::MailFolders(cmd) => cmd.execute(printer, account, client),
            Self::Messages(cmd) => cmd.execute(printer, account, client),
            Self::Attachments(cmd) => cmd.execute(printer, account, client),
        }
    }
}
