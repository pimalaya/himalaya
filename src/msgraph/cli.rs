use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::account::context::Account;
use crate::msgraph::{
    attachments::MsgraphAttachmentsCommand, client::MsgraphClient,
    mail_folders::MsgraphMailFoldersCommand, messages::MsgraphMessagesCommand,
    profile::MsgraphProfileCommand,
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
    #[command(subcommand)]
    #[command(visible_aliases = ["mail-folder", "folders", "folder"])]
    MailFolders(MsgraphMailFoldersCommand),
    #[command(subcommand)]
    #[command(visible_aliases = ["message", "msg"])]
    Messages(MsgraphMessagesCommand),
    #[command(subcommand)]
    #[command(visible_aliases = ["attachment"])]
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
