#[cfg(feature = "flag-add")]
mod add;
#[cfg(feature = "flag-remove")]
mod remove;
#[cfg(feature = "flag-set")]
mod set;

use anyhow::Result;
use clap::Subcommand;

use crate::{config::TomlConfig, printer::Printer};

#[cfg(feature = "flag-add")]
use self::add::FlagAddCommand;
#[cfg(feature = "flag-remove")]
use self::remove::FlagRemoveCommand;
#[cfg(feature = "flag-set")]
use self::set::FlagSetCommand;

/// Manage flags.
///
/// A flag is a tag associated to an envelope. Existing flags are
/// seen, answered, flagged, deleted, draft. Other flags are
/// considered custom, which are not always supported (the
/// synchronization does not take care of them yet).
#[derive(Debug, Subcommand)]
pub enum FlagSubcommand {
    #[cfg(feature = "flag-add")]
    #[command(arg_required_else_help = true)]
    #[command(alias = "create")]
    Add(FlagAddCommand),

    #[cfg(feature = "flag-set")]
    #[command(arg_required_else_help = true)]
    #[command(aliases = ["update", "change", "replace"])]
    Set(FlagSetCommand),

    #[cfg(feature = "flag-remove")]
    #[command(arg_required_else_help = true)]
    #[command(aliases = ["rm", "delete", "del"])]
    Remove(FlagRemoveCommand),
}

impl FlagSubcommand {
    #[allow(unused)]
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        match self {
            #[cfg(feature = "flag-add")]
            Self::Add(cmd) => cmd.execute(printer, config).await,
            #[cfg(feature = "flag-set")]
            Self::Set(cmd) => cmd.execute(printer, config).await,
            #[cfg(feature = "flag-remove")]
            Self::Remove(cmd) => cmd.execute(printer, config).await,
        }
    }
}
