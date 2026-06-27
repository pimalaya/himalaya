use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::smtp::{client::SmtpClient, raw::SmtpRawCommand, send::SmtpSendCommand};

/// SMTP-specific API.
///
/// Gives access to the raw SMTP API. Every CLI command matches the name of its
/// SMTP counterpart.
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum SmtpCommand {
    Send(SmtpSendCommand),

    // Raw passthrough.
    Raw(SmtpRawCommand),
}

impl SmtpCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut SmtpClient) -> Result<()> {
        match self {
            Self::Send(cmd) => cmd.execute(printer, client),

            Self::Raw(cmd) => cmd.execute(printer, client),
        }
    }
}
