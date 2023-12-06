pub mod list;

use anyhow::Result;
use clap::Subcommand;

use crate::{config::TomlConfig, printer::Printer};

use self::list::EnvelopeListCommand;

/// Subcommand to manage envelopes
#[derive(Debug, Subcommand)]
pub enum EnvelopeSubcommand {
    /// List all envelopes from a folder
    #[command(alias = "lst")]
    List(EnvelopeListCommand),
}

impl EnvelopeSubcommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, config).await,
        }
    }
}
