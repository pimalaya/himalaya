use anyhow::{bail, Result};
use clap::Parser;
use io_imap::{
    coroutines::{r#move::*, select::*},
    types::mailbox::Mailbox,
};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::imap::{
    account::ImapAccount,
    mailbox::arg::{MailboxNameOptionalFlag, MailboxSelectFlag, TargetMailboxNameArg},
    stream,
};

/// Move message(s) to the given mailbox.
///
/// This command moves messages identified by the given sequence set
/// from the source mailbox to the destination mailbox. Requires the
/// MOVE IMAP extension.
#[derive(Debug, Parser)]
pub struct MoveMessageCommand {
    #[command(flatten)]
    pub mailbox: MailboxNameOptionalFlag,
    #[command(flatten)]
    pub select: MailboxSelectFlag,

    /// The sequence set of messages (e.g., "1", "1,2,3", "1:*").
    #[arg(name = "sequence_set", value_name = "SEQUENCE")]
    pub sequence_set: String,
    #[command(flatten)]
    pub destination: TargetMailboxNameArg,

    /// Use sequence numbers instead of UIDs.
    #[arg(long)]
    pub seq: bool,
}

impl MoveMessageCommand {
    pub fn exec(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let (mut context, mut stream) = stream::connect(account.backend)?;

        let mailbox = self.mailbox.name.try_into()?;

        if self.select.r#true {
            let mut arg = None;
            let mut coroutine = ImapSelect::new(context, mailbox);

            context = loop {
                match coroutine.resume(arg.take()) {
                    ImapSelectResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                    ImapSelectResult::Ok { context, .. } => break context,
                    ImapSelectResult::Err { err, .. } => bail!(err),
                }
            };
        }

        let sequence_set = self.sequence_set.as_str().try_into()?;
        let destination: Mailbox<'static> = self.destination.name.try_into()?;

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
