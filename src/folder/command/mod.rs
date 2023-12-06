mod create;
mod delete;
mod expunge;
mod list;
mod purge;

use anyhow::Result;
use clap::Subcommand;

use crate::{config::TomlConfig, printer::Printer};

use self::{
    create::FolderCreateCommand, delete::FolderDeleteCommand, expunge::FolderExpungeCommand,
    list::FolderListCommand, purge::FolderPurgeCommand,
};

/// Subcommand to manage accounts
#[derive(Debug, Subcommand)]
pub enum FolderSubcommand {
    /// Create a new folder
    #[command(alias = "add")]
    Create(FolderCreateCommand),

    /// List all folders
    #[command(alias = "lst")]
    List(FolderListCommand),

    /// Expunge a folder
    #[command()]
    Expunge(FolderExpungeCommand),

    /// Purge a folder
    #[command()]
    Purge(FolderPurgeCommand),

    /// Delete a folder
    #[command(alias = "remove", alias = "rm")]
    Delete(FolderDeleteCommand),
}

impl FolderSubcommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        match self {
            Self::Create(cmd) => cmd.execute(printer, config).await,
            Self::List(cmd) => cmd.execute(printer, config).await,
            Self::Expunge(cmd) => cmd.execute(printer, config).await,
            Self::Purge(cmd) => cmd.execute(printer, config).await,
            Self::Delete(cmd) => cmd.execute(printer, config).await,
        }
    }
}
