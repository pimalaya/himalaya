mod download;

use color_eyre::Result;
use clap::Subcommand;

use crate::{config::TomlConfig, printer::Printer};

use self::download::AttachmentDownloadCommand;

/// Manage attachments.
///
/// A message body can be composed of multiple MIME parts. An
/// attachment is the representation of a binary part of a message
/// body.
#[derive(Debug, Subcommand)]
pub enum AttachmentSubcommand {
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
