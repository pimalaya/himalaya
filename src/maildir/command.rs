use anyhow::Result;
use clap::Subcommand;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::maildir::{account::MaildirAccount, message::command::MessageCommand};

/// MAILDIR CLI (requires the `maildir` cargo feature).
///
/// This command gives you access to the MAILDIR CLI API, and allows you
/// to manage MAILDIR mailboxes, envelopes, flags, messages etc.
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum MaildirCommand {
    #[command(subcommand)]
    #[command(aliases = ["msgs", "msg"])]
    Messages(MessageCommand),
}

impl MaildirCommand {
    pub fn execute(self, printer: &mut impl Printer, account: MaildirAccount) -> Result<()> {
        match self {
            Self::Messages(cmd) => cmd.execute(printer, account),
        }
    }
}
