use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    cli::BackendArg,
    config::{AccountConfig, Config},
    flags::{add::FlagsAddCommand, delete::FlagsDeleteCommand, set::FlagsSetCommand},
};

/// Manage flags through whichever backend the active account has
/// configured.
///
/// The active backend is selected by `--backend` (defaults to `auto`,
/// which picks the first configured backend in priority order).
#[derive(Debug, Subcommand)]
pub enum FlagsCommand {
    Add(FlagsAddCommand),
    Set(FlagsSetCommand),
    #[command(visible_alias = "remove", visible_alias = "rm")]
    Delete(FlagsDeleteCommand),
}

impl FlagsCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        config: Config,
        account_config: AccountConfig,
        backend: BackendArg,
    ) -> Result<()> {
        match self {
            Self::Add(cmd) => cmd.execute(printer, config, account_config, backend),
            Self::Set(cmd) => cmd.execute(printer, config, account_config, backend),
            Self::Delete(cmd) => cmd.execute(printer, config, account_config, backend),
        }
    }
}
