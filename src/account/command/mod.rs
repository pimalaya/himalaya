mod configure;
mod list;
mod sync;

use anyhow::Result;
use clap::Subcommand;

use crate::{config::TomlConfig, printer::Printer};

/// Subcommand to manage accounts
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Configure the given account
    #[command(alias = "cfg")]
    Configure(configure::Command),

    /// List all exsting accounts
    #[command(alias = "lst")]
    List(list::Command),

    /// Synchronize the given account locally
    #[command()]
    Sync(sync::Command),
}

impl Command {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        match self {
            Self::Configure(cmd) => cmd.execute(printer, config).await,
            Self::List(cmd) => cmd.execute(printer, config).await,
            Self::Sync(cmd) => cmd.execute(printer, config).await,
        }
    }
}
