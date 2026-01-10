mod configure;
mod doctor;
mod list;

#[cfg(feature = "oauth2")]
mod auth;

use std::path::PathBuf;

use clap::Subcommand;
use color_eyre::Result;
use pimalaya_tui::terminal::cli::printer::Printer;

use crate::config::TomlConfig;

use self::{
    configure::AccountConfigureCommand, doctor::AccountDoctorCommand, list::AccountListCommand,
};

#[cfg(feature = "oauth2")]
use self::auth::AccountAuthCommand;

/// Configure, list and diagnose your accounts.
///
/// An account is a group of settings, identified by a unique
/// name. This subcommand allows you to manage your accounts.
#[derive(Debug, Subcommand)]
pub enum AccountSubcommand {
    #[cfg(feature = "oauth2")]
    Auth(AccountAuthCommand),
    Configure(AccountConfigureCommand),
    Doctor(AccountDoctorCommand),
    List(AccountListCommand),
}

impl AccountSubcommand {
    pub async fn execute(
        self,
        printer: &mut impl Printer,
        config: TomlConfig,
        config_path: Option<&PathBuf>,
    ) -> Result<()> {
        match self {
            #[cfg(feature = "oauth2")]
            Self::Auth(cmd) => cmd.execute(config, config_path).await,
            Self::Configure(cmd) => cmd.execute(config, config_path).await,
            Self::Doctor(cmd) => cmd.execute(&config).await,
            Self::List(cmd) => cmd.execute(printer, &config).await,
        }
    }
}
