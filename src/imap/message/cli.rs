use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::imap::{
    client::ImapClient,
    message::{
        copy::ImapMessageCopyCommand, get::ImapMessageGetCommand, r#move::ImapMessageMoveCommand,
        save::ImapMessageSaveCommand,
    },
};

/// Manage IMAP messages.
///
/// A message is a complete email including headers and body. This
/// subcommand stores and relocates messages (`save`, `copy`, `move`)
/// and shows their structure (`get`); reading bodies and extracting
/// parts is the job of the shared `messages` and `attachments`
/// commands.
#[derive(Debug, Subcommand)]
pub enum ImapMessageCommand {
    Save(ImapMessageSaveCommand),
    Get(ImapMessageGetCommand),
    Copy(ImapMessageCopyCommand),
    Move(ImapMessageMoveCommand),
}

impl ImapMessageCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        match self {
            Self::Save(cmd) => cmd.execute(printer, client),
            Self::Get(cmd) => cmd.execute(printer, client),
            Self::Copy(cmd) => cmd.execute(printer, client),
            Self::Move(cmd) => cmd.execute(printer, client),
        }
    }
}
