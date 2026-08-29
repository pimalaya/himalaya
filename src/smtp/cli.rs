//! # SMTP command
//!
//! The `smtp` command, dispatching onto its subcommands.

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
    /// Send a raw RFC 5322 message (MAIL FROM / RCPT TO / DATA).
    Send(SmtpSendCommand),
    /// Send a raw SMTP command and print the verbatim reply.
    Raw(SmtpRawCommand),
}

impl SmtpCommand {
    /// Runs the subcommand against the account's SMTP session.
    pub fn execute(self, printer: &mut impl Printer, client: &mut SmtpClient) -> Result<()> {
        match self {
            Self::Send(cmd) => cmd.execute(printer, client),

            Self::Raw(cmd) => cmd.execute(printer, client),
        }
    }
}
