//! # Microsoft Graph mail folders command
//!
//! The `msgraph mail-folders` command, dispatching onto its subcommands.

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    msgraph::{
        client::MsgraphClient,
        mail_folders::{
            child_folders::MsgraphChildFoldersListCommand, copy::MsgraphMailFolderCopyCommand,
            create::MsgraphMailFolderCreateCommand, delete::MsgraphMailFolderDeleteCommand,
            get::MsgraphMailFolderGetCommand, list::MsgraphMailFolderListCommand,
            r#move::MsgraphMailFolderMoveCommand, rename::MsgraphMailFolderRenameCommand,
        },
    },
};

/// Manage Microsoft Graph mail folders (`me.mailFolders`).
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum MsgraphMailFoldersCommand {
    List(MsgraphMailFolderListCommand),
    #[command(visible_alias = "children", aliases = ["child-folder", "child"])]
    ChildFolders(MsgraphChildFoldersListCommand),
    Get(MsgraphMailFolderGetCommand),
    Create(MsgraphMailFolderCreateCommand),
    Rename(MsgraphMailFolderRenameCommand),
    Copy(MsgraphMailFolderCopyCommand),
    #[command(name = "move")]
    Move(MsgraphMailFolderMoveCommand),
    #[command(visible_aliases = ["del", "remove", "rm"])]
    Delete(MsgraphMailFolderDeleteCommand),
}

impl MsgraphMailFoldersCommand {
    /// Runs the subcommand against the account's Graph client.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut MsgraphClient,
    ) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, account, client),
            Self::ChildFolders(cmd) => cmd.execute(printer, account, client),
            Self::Get(cmd) => cmd.execute(printer, account, client),
            Self::Create(cmd) => cmd.execute(printer, client),
            Self::Rename(cmd) => cmd.execute(printer, client),
            Self::Copy(cmd) => cmd.execute(printer, client),
            Self::Move(cmd) => cmd.execute(printer, client),
            Self::Delete(cmd) => cmd.execute(printer, client),
        }
    }
}
