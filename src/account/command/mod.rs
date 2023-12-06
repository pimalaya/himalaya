mod configure;
mod list;
mod sync;

use anyhow::Result;
use clap::Subcommand;

use crate::{config::TomlConfig, printer::Printer};

use self::{
    configure::AccountConfigureCommand, list::AccountListCommand, sync::AccountSyncCommand,
};

/// Subcommand to manage accounts
#[derive(Debug, Subcommand)]
pub enum AccountSubcommand {
    /// Configure an account
    #[command(alias = "cfg")]
    Configure(AccountConfigureCommand),

    /// List all accounts
    #[command(alias = "lst")]
    List(AccountListCommand),

    /// Synchronize an account locally
    #[command()]
    Sync(AccountSyncCommand),
}

impl AccountSubcommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        match self {
            Self::Configure(cmd) => cmd.execute(printer, config).await,
            Self::List(cmd) => cmd.execute(printer, config).await,
            Self::Sync(cmd) => cmd.execute(printer, config).await,
        }
    }
}
