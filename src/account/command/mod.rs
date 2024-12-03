mod configure;
mod doctor;
mod list;

use std::path::PathBuf;

use clap::Subcommand;
use color_eyre::Result;
use pimalaya_tui::terminal::cli::printer::Printer;

use crate::config::TomlConfig;

use self::{
    configure::AccountConfigureCommand, doctor::AccountDoctorCommand, list::AccountListCommand,
};

/// Manage accounts.
///
/// An account is a set of settings, identified by an account
/// name. Settings are directly taken from your TOML configuration
/// file. This subcommand allows you to manage them.
#[derive(Debug, Subcommand)]
pub enum AccountSubcommand {
    #[command(alias = "cfg")]
    Configure(AccountConfigureCommand),

    #[command()]
    Doctor(AccountDoctorCommand),

    #[command(alias = "lst")]
    List(AccountListCommand),
}

impl AccountSubcommand {
    #[allow(unused)]
    pub async fn execute(
        self,
        printer: &mut impl Printer,
        config: TomlConfig,
        config_path: Option<&PathBuf>,
    ) -> Result<()> {
        match self {
            Self::Configure(cmd) => cmd.execute(config, config_path).await,
            Self::Doctor(cmd) => cmd.execute(&config).await,
            Self::List(cmd) => cmd.execute(printer, &config).await,
        }
    }
}
