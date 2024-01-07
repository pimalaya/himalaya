#[cfg(feature = "account-configure")]
mod configure;
#[cfg(feature = "account-list")]
mod list;
#[cfg(feature = "sync")]
mod sync;

use anyhow::Result;
use clap::Subcommand;

use crate::{config::TomlConfig, printer::Printer};

#[cfg(feature = "account-configure")]
use self::configure::AccountConfigureCommand;
#[cfg(feature = "account-list")]
use self::list::AccountListCommand;
#[cfg(feature = "sync")]
use self::sync::AccountSyncCommand;

/// Manage accounts.
///
/// An account is a set of settings, identified by an account
/// name. Settings are directly taken from your TOML configuration
/// file. This subcommand allows you to manage them.
#[derive(Debug, Subcommand)]
pub enum AccountSubcommand {
    #[cfg(feature = "account-configure")]
    #[command(alias = "cfg")]
    Configure(AccountConfigureCommand),

    #[cfg(feature = "account-list")]
    #[command(alias = "lst")]
    List(AccountListCommand),

    #[cfg(feature = "sync")]
    #[command(alias = "synchronize", alias = "synchronise")]
    Sync(AccountSyncCommand),
}

impl AccountSubcommand {
    #[allow(unused)]
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        match self {
            #[cfg(feature = "account-configure")]
            Self::Configure(cmd) => cmd.execute(printer, config).await,
            #[cfg(feature = "account-list")]
            Self::List(cmd) => cmd.execute(printer, config).await,
            #[cfg(feature = "sync")]
            Self::Sync(cmd) => cmd.execute(printer, config).await,
        }
    }
}
