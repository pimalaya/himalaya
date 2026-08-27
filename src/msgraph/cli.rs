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

/// Microsoft Graph CLI.
///
/// This command gives you access to the Microsoft Graph REST API,
/// organized by Graph resource: the signed-in user (profile), mail
/// folders, messages and message attachments.
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
