mod download;

use clap::Subcommand;
use color_eyre::Result;
use pimalaya_tui::terminal::cli::printer::Printer;

use crate::config::TomlConfig;

use self::download::AttachmentDownloadCommand;

/// Download your message attachments.
///
/// A message body can be composed of multiple MIME parts. An
/// attachment is the representation of a binary part of a message
/// body.
#[derive(Debug, Subcommand)]
pub enum AttachmentSubcommand {
    #[command(arg_required_else_help = true, alias = "dl")]
    Download(AttachmentDownloadCommand),
}

impl AttachmentSubcommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        match self {
            Self::Download(cmd) => cmd.execute(printer, config).await,
        }
    }
}
