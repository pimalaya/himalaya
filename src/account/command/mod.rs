mod configure;
mod list;
#[cfg(feature = "account-sync")]
mod sync;

use anyhow::Result;
use clap::Subcommand;

use crate::{config::TomlConfig, printer::Printer};

#[cfg(feature = "account-sync")]
use self::sync::AccountSyncCommand;
use self::{configure::AccountConfigureCommand, list::AccountListCommand};

/// Manage accounts.
///
/// An account is a set of settings, identified by an account
/// name. Settings are directly taken from your TOML configuration
/// file. This subcommand allows you to manage them.
#[derive(Debug, Subcommand)]
pub enum AccountSubcommand {
    #[command(alias = "cfg")]
    Configure(AccountConfigureCommand),

    #[command(alias = "lst")]
    List(AccountListCommand),

    #[cfg(feature = "account-sync")]
    #[command(alias = "synchronize", alias = "synchronise")]
    Sync(AccountSyncCommand),
}

impl AccountSubcommand {
    #[allow(unused)]
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        match self {
            Self::Configure(cmd) => cmd.execute(printer, config).await,
            Self::List(cmd) => cmd.execute(printer, config).await,
            #[cfg(feature = "account-sync")]
            Self::Sync(cmd) => cmd.execute(printer, config).await,
        }
    }
}
