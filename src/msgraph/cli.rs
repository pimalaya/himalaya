use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::account::context::Account;
use crate::msgraph::{
    attachment::cli::MsgraphAttachmentCommand, client::MsgraphClient,
    mail_folder::cli::MsgraphMailFolderCommand, message::cli::MsgraphMessageCommand,
    profile::cli::MsgraphProfileCommand,
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
    #[command(subcommand, visible_aliases = ["mail-folders", "folder", "folders"])]
    MailFolder(MsgraphMailFolderCommand),
    #[command(subcommand, visible_aliases = ["messages", "msg"])]
    Message(MsgraphMessageCommand),
    #[command(subcommand, visible_aliases = ["attachments"])]
    Attachment(MsgraphAttachmentCommand),
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
            Self::MailFolder(cmd) => cmd.execute(printer, account, client),
            Self::Message(cmd) => cmd.execute(printer, account, client),
            Self::Attachment(cmd) => cmd.execute(printer, account, client),
        }
    }
}
