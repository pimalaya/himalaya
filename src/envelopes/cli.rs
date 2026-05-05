use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    cli::BackendArg,
    config::{AccountConfig, Config},
    envelopes::list::EnvelopesListCommand,
};

/// List envelopes through whichever backend the active account has
/// configured.
///
/// The active backend is selected by `--backend` (defaults to `auto`,
/// which picks the first configured backend in priority order).
#[derive(Debug, Subcommand)]
pub enum EnvelopesCommand {
    #[command(visible_alias = "ls")]
    List(EnvelopesListCommand),
}

impl EnvelopesCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        config: Config,
        account_config: AccountConfig,
        backend: BackendArg,
    ) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, config, account_config, backend),
        }
    }
}
