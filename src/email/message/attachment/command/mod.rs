pub mod download;

use anyhow::Result;
use clap::Subcommand;

use crate::{config::TomlConfig, printer::Printer};

use self::download::AttachmentDownloadCommand;

/// Subcommand dedicated to attachments
#[derive(Debug, Subcommand)]
pub enum AttachmentSubcommand {
    /// Download all attachments of one or more messages
    #[command(arg_required_else_help = true)]
    Download(AttachmentDownloadCommand),
}

impl AttachmentSubcommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        match self {
            Self::Download(cmd) => cmd.execute(printer, config).await,
        }
    }
}
