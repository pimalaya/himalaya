use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::smtp::{client::SmtpClient, message::cli::SmtpMessageCommand};

/// SMTP CLI.
///
/// This command gives you access to the SMTP CLI API, and allows
/// you to manage SMTP mailboxes: list mailboxes, read messages,
/// add flags etc.
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum SmtpCommand {
    #[command(subcommand)]
    #[command(aliases = ["msgs", "msg"])]
    Messages(SmtpMessageCommand),
}

impl SmtpCommand {
    pub fn execute(self, printer: &mut impl Printer, client: SmtpClient) -> Result<()> {
        match self {
            Self::Messages(cmd) => cmd.execute(printer, client),
        }
    }
}
