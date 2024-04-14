mod add;
mod delete;
mod expunge;
mod list;
mod purge;

use color_eyre::Result;
use clap::Subcommand;

use crate::{config::TomlConfig, printer::Printer};

use self::{
    add::AddFolderCommand, delete::FolderDeleteCommand, expunge::FolderExpungeCommand,
    list::FolderListCommand, purge::FolderPurgeCommand,
};

/// Manage folders.
///
/// A folder (as known as mailbox, or directory) contains one or more
/// emails. This subcommand allows you to manage them.
#[derive(Debug, Subcommand)]
pub enum FolderSubcommand {
    #[command(visible_alias = "create", alias = "new")]
    Add(AddFolderCommand),

    #[command(alias = "lst")]
    List(FolderListCommand),

    #[command()]
    Expunge(FolderExpungeCommand),

    #[command()]
    Purge(FolderPurgeCommand),

    #[command(alias = "remove", alias = "rm")]
    Delete(FolderDeleteCommand),
}

impl FolderSubcommand {
    #[allow(unused)]
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        match self {
            Self::Add(cmd) => cmd.execute(printer, config).await,
            Self::List(cmd) => cmd.execute(printer, config).await,
            Self::Expunge(cmd) => cmd.execute(printer, config).await,
            Self::Purge(cmd) => cmd.execute(printer, config).await,
            Self::Delete(cmd) => cmd.execute(printer, config).await,
        }
    }
}
