use anyhow::{bail, Result};
use clap::Parser;
use io_imap::{
    coroutines::{r#move::*, select::*},
    types::mailbox::Mailbox,
};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::{config::ImapConfig, imap::mailbox::arg::MailboxNameOptionalFlag, imap::stream};

/// Move messages to another mailbox.
///
/// This command moves messages identified by the given sequence set
/// from the source mailbox to the destination mailbox. Requires the
/// MOVE IMAP extension.
#[derive(Debug, Parser)]
pub struct MoveMessageCommand {
    #[command(flatten)]
    pub mailbox: MailboxNameOptionalFlag,

    /// The sequence set of messages (e.g., "1", "1,2,3", "1:*").
    #[arg(name = "sequence_set", value_name = "SEQUENCE")]
    pub sequence_set: String,

    /// The destination mailbox.
    #[arg(name = "destination", value_name = "DESTINATION")]
    pub destination: String,

    /// Use sequence numbers instead of UIDs.
    #[arg(long)]
    pub seq: bool,
}

impl MoveMessageCommand {
    pub fn exec(self, printer: &mut impl Printer, config: ImapConfig) -> Result<()> {
        let (context, mut stream) = stream::connect(config)?;

        let mailbox = self.mailbox.name.try_into()?;

        // SELECT mailbox
        let mut arg = None;
        let mut coroutine = ImapSelect::new(context, mailbox);

        let context = loop {
            match coroutine.resume(arg.take()) {
                ImapSelectResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                ImapSelectResult::Ok { context, .. } => break context,
                ImapSelectResult::Err { err, .. } => bail!(err),
            }
        };

        // Parse sequence set and destination
        let sequence_set = self.sequence_set.as_str().try_into()?;
        let destination: Mailbox<'static> = self.destination.try_into()?;

        // MOVE
        let mut arg = None;
        let mut coroutine = ImapMove::new(context, sequence_set, destination, !self.seq);

        loop {
            match coroutine.resume(arg.take()) {
                ImapMoveResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                ImapMoveResult::Ok { .. } => break,
                ImapMoveResult::Err { err, .. } => bail!(err),
            }
        }

        printer.out(Message::new("Message(s) successfully moved"))
    }
}
