mod check_up;
mod configure;
mod list;

use clap::Subcommand;
use color_eyre::Result;

use crate::{config::TomlConfig, printer::Printer};

use self::{
    check_up::AccountCheckUpCommand, configure::AccountConfigureCommand, list::AccountListCommand,
};

/// Manage accounts.
///
/// An account is a set of settings, identified by an account
/// name. Settings are directly taken from your TOML configuration
/// file. This subcommand allows you to manage them.
#[derive(Debug, Subcommand)]
pub enum AccountSubcommand {
    #[command(alias = "checkup")]
    CheckUp(AccountCheckUpCommand),

    #[command(alias = "cfg")]
    Configure(AccountConfigureCommand),

    #[command(alias = "lst")]
    List(AccountListCommand),
}

impl AccountSubcommand {
    #[allow(unused)]
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        match self {
            Self::CheckUp(cmd) => cmd.execute(printer, config).await,
            Self::Configure(cmd) => cmd.execute(printer, config).await,
            Self::List(cmd) => cmd.execute(printer, config).await,
        }
    }
}
