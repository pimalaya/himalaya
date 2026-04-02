use anyhow::{bail, Result};
use clap::Parser;
use io_imap::{rfc3501::select::*, rfc6851::r#move::*, types::mailbox::Mailbox};
use io_socket::runtimes::std_stream::handle;
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
pub struct ImapMessageMoveCommand {
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

impl ImapMessageMoveCommand {
    pub fn execute(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let mut imap = account.new_imap_session()?;
        let mailbox = self.mailbox_name.inner.try_into()?;

        if !self.mailbox_no_select.inner {
            let mut arg = None;
            let mut coroutine = ImapMailboxSelect::new(imap.context, mailbox);

            imap.context = loop {
                match coroutine.resume(arg.take()) {
                    ImapMailboxSelectResult::Io { input } => {
                        arg = Some(handle(&mut imap.stream, input)?)
                    }
                    ImapMailboxSelectResult::Ok { context, .. } => break context,
                    ImapMailboxSelectResult::Err { err, .. } => bail!(err),
                }
            };
        }

        let sequence_set = self.sequence_set.as_str().try_into()?;
        let destination: Mailbox<'static> = self.mailbox_dest_name.inner.try_into()?;

        let mut arg = None;
        let mut coroutine =
            ImapMessageMove::new(imap.context, sequence_set, destination, !self.seq);

        loop {
            match coroutine.resume(arg.take()) {
                ImapMessageMoveResult::Io { input } => arg = Some(handle(&mut imap.stream, input)?),
                ImapMessageMoveResult::Ok { .. } => break,
                ImapMessageMoveResult::Err { err, .. } => bail!(err),
            }
        }

        printer.out(Message::new("Message(s) successfully moved"))
    }
}
