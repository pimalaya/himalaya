use anyhow::Result;
use clap::Subcommand;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::{
    attachments::{download::AttachmentsDownloadCommand, list::AttachmentsListCommand},
    cli::BackendArg,
    config::{AccountConfig, Config},
};

/// List or download attachments carried by a single message.
///
/// Available wherever `messages get` is — that is, IMAP, JMAP and
/// Maildir. The active backend is selected by `--backend` (default
/// `auto`).
#[derive(Debug, Subcommand)]
pub enum AttachmentsCommand {
    List(AttachmentsListCommand),
    Download(AttachmentsDownloadCommand),
}

impl AttachmentsCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        config: Config,
        account_config: AccountConfig,
        backend: BackendArg,
    ) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, config, account_config, backend),
            Self::Download(cmd) => cmd.execute(printer, config, account_config, backend),
        }
    }
}
