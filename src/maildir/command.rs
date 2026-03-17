use anyhow::Result;
use clap::Subcommand;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::maildir::{
    account::MaildirAccount, create::CreateMaildirCommand, delete::DeleteMaildirCommand,
    envelope::command::EnvelopesCommand, flag::command::FlagCommand, list::ListMaildirsCommand,
    message::command::MessageCommand, rename::RenameMaildirCommand,
};

/// MAILDIR CLI (requires the `maildir` cargo feature).
///
/// This command gives you access to the MAILDIR CLI API, and allows you
/// to manage MAILDIR mailboxes, envelopes, flags, messages etc.
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum MaildirCommand {
    Create(CreateMaildirCommand),
    Rename(RenameMaildirCommand),
    Delete(DeleteMaildirCommand),
    List(ListMaildirsCommand),

    #[command(subcommand)]
    #[command(aliases = ["msgs", "msg"])]
    Messages(MessageCommand),
    #[command(subcommand)]
    Flags(FlagCommand),
    #[command(subcommand)]
    Envelopes(EnvelopesCommand),
}

impl MaildirCommand {
    pub fn execute(self, printer: &mut impl Printer, account: MaildirAccount) -> Result<()> {
        match self {
            Self::Create(cmd) => cmd.execute(printer, account),
            Self::Rename(cmd) => cmd.execute(printer, account),
            Self::Delete(cmd) => cmd.execute(printer, account),
            Self::List(cmd) => cmd.execute(printer, account),

            Self::Messages(cmd) => cmd.execute(printer, account),
            Self::Flags(cmd) => cmd.execute(printer, account),
            Self::Envelopes(cmd) => cmd.execute(printer, account),
        }
    }
}
