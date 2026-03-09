use anyhow::Result;
use clap::Subcommand;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::{config::SmtpConfig, smtp::message::command::MessageCommand};

/// SMTP CLI (requires `smtp` cargo feature).
///
/// This command gives you access to the SMTP CLI API, and allows
/// you to manage SMTP mailboxes: list mailboxes, read messages,
/// add flags etc.
#[derive(Debug, Subcommand)]
#[command(rename_all = "lowercase")]
pub enum SmtpCommand {
    #[command(subcommand)]
    #[command(aliases = ["message", "msg"])]
    Messages(MessageCommand),
}

impl SmtpCommand {
    pub fn execute(self, printer: &mut impl Printer, config: SmtpConfig) -> Result<()> {
        match self {
            Self::Messages(cmd) => cmd.execute(printer, config),
        }
    }
}
