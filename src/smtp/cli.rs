use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::smtp::{account::SmtpAccount, message::cli::SmtpMessageCommand};

/// SMTP CLI (requires `smtp` cargo feature).
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
    pub fn execute(self, printer: &mut impl Printer, account: SmtpAccount) -> Result<()> {
        match self {
            Self::Messages(cmd) => cmd.execute(printer, account),
        }
    }
}
