mod add;
mod remove;
mod set;

use color_eyre::Result;
use clap::Subcommand;

use crate::{config::TomlConfig, printer::Printer};

use self::{add::FlagAddCommand, remove::FlagRemoveCommand, set::FlagSetCommand};

/// Manage flags.
///
/// A flag is a tag associated to an envelope. Existing flags are
/// seen, answered, flagged, deleted, draft. Other flags are
/// considered custom, which are not always supported (the
/// synchronization does not take care of them yet).
#[derive(Debug, Subcommand)]
pub enum FlagSubcommand {
    #[command(arg_required_else_help = true)]
    #[command(alias = "create")]
    Add(FlagAddCommand),

    #[command(arg_required_else_help = true)]
    #[command(aliases = ["update", "change", "replace"])]
    Set(FlagSetCommand),

    #[command(arg_required_else_help = true)]
    #[command(aliases = ["rm", "delete", "del"])]
    Remove(FlagRemoveCommand),
}

impl FlagSubcommand {
    #[allow(unused)]
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        match self {
            Self::Add(cmd) => cmd.execute(printer, config).await,
            Self::Set(cmd) => cmd.execute(printer, config).await,
            Self::Remove(cmd) => cmd.execute(printer, config).await,
        }
    }
}
