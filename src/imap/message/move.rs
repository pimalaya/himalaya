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
    mailbox::arg::{MailboxNameOptionalFlag, MailboxNoSelectFlag, TargetMailboxNameArg},
};

/// Move message(s) to the given mailbox.
///
/// This command moves messages identified by the given sequence set
/// from the source mailbox to the destination mailbox. Requires the
/// MOVE IMAP extension.
#[derive(Debug, Parser)]
pub struct MoveMessageCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameOptionalFlag,
    #[command(flatten)]
    pub mailbox_no_select: MailboxNoSelectFlag,

    /// The sequence set of messages (e.g., "1", "1,2,3", "1:*").
    #[arg(name = "sequence_set", value_name = "SEQUENCE")]
    pub sequence_set: String,
    #[command(flatten)]
    pub mailbox_dest_name: TargetMailboxNameArg,

    /// Use sequence numbers instead of UIDs.
    #[arg(long)]
    pub seq: bool,
}

impl MoveMessageCommand {
    pub fn execute(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let mut imap = account.new_imap_session()?;
        let mailbox = self.mailbox_name.inner.try_into()?;

        if !self.mailbox_no_select.inner {
            let mut arg = None;
            let mut coroutine = ImapSelect::new(imap.context, mailbox);

            imap.context = loop {
                match coroutine.resume(arg.take()) {
                    ImapSelectResult::Io { io } => arg = Some(handle(&mut imap.stream, io)?),
                    ImapSelectResult::Ok { context, .. } => break context,
                    ImapSelectResult::Err { err, .. } => bail!(err),
                }
            };
        }

        let sequence_set = self.sequence_set.as_str().try_into()?;
        let destination: Mailbox<'static> = self.mailbox_dest_name.inner.try_into()?;

        let mut arg = None;
        let mut coroutine = ImapMove::new(imap.context, sequence_set, destination, !self.seq);

        loop {
            match coroutine.resume(arg.take()) {
                ImapMoveResult::Io { io } => arg = Some(handle(&mut imap.stream, io)?),
                ImapMoveResult::Ok { .. } => break,
                ImapMoveResult::Err { err, .. } => bail!(err),
            }
        }

        printer.out(Message::new("Message(s) successfully moved"))
    }
}
