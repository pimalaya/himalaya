use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::smtp::{client::SmtpClient, message::cli::SmtpMessageCommand};

/// SMTP-specific API.
///
/// Gives access to the raw SMTP API. Every CLI command matches the name of its
/// SMTP counterpart.
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum SmtpCommand {
    #[command(subcommand)]
    #[command(aliases = ["msgs", "msg"])]
    Messages(SmtpMessageCommand),
}

impl SmtpCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut SmtpClient) -> Result<()> {
        match self {
            Self::Messages(cmd) => cmd.execute(printer, client),
        }
    }
}
