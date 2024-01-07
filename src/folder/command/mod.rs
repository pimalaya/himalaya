#[cfg(feature = "folder-add")]
mod create;
#[cfg(feature = "folder-delete")]
mod delete;
#[cfg(feature = "folder-expunge")]
mod expunge;
#[cfg(feature = "folder-list")]
mod list;
#[cfg(feature = "folder-purge")]
mod purge;

use anyhow::Result;
use clap::Subcommand;

use crate::{config::TomlConfig, printer::Printer};

#[cfg(feature = "folder-add")]
use self::create::AddFolderCommand;
#[cfg(feature = "folder-delete")]
use self::delete::FolderDeleteCommand;
#[cfg(feature = "folder-expunge")]
use self::expunge::FolderExpungeCommand;
#[cfg(feature = "folder-list")]
use self::list::FolderListCommand;
#[cfg(feature = "folder-purge")]
use self::purge::FolderPurgeCommand;

/// Manage folders.
///
/// A folder (as known as mailbox, or directory) contains one or more
/// emails. This subcommand allows you to manage them.
#[derive(Debug, Subcommand)]
pub enum FolderSubcommand {
    #[cfg(feature = "folder-add")]
    #[command(visible_alias = "create", alias = "new")]
    Add(AddFolderCommand),

    #[cfg(feature = "folder-list")]
    #[command(alias = "lst")]
    List(FolderListCommand),

    #[cfg(feature = "folder-expunge")]
    #[command()]
    Expunge(FolderExpungeCommand),

    #[cfg(feature = "folder-purge")]
    #[command()]
    Purge(FolderPurgeCommand),

    #[cfg(feature = "folder-delete")]
    #[command(alias = "remove", alias = "rm")]
    Delete(FolderDeleteCommand),
}

impl FolderSubcommand {
    #[allow(unused)]
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        match self {
            #[cfg(feature = "folder-add")]
            Self::Add(cmd) => cmd.execute(printer, config).await,
            #[cfg(feature = "folder-list")]
            Self::List(cmd) => cmd.execute(printer, config).await,
            #[cfg(feature = "folder-expunge")]
            Self::Expunge(cmd) => cmd.execute(printer, config).await,
            #[cfg(feature = "folder-purge")]
            Self::Purge(cmd) => cmd.execute(printer, config).await,
            #[cfg(feature = "folder-delete")]
            Self::Delete(cmd) => cmd.execute(printer, config).await,
        }
    }
}
