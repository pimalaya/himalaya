use anyhow::{bail, Result};
use clap::Parser;
use io_imap::{
    rfc3501::{select::*, store::*},
    types::{
        flag::{Flag, StoreType},
        IntoStatic,
    },
};
use io_socket::runtimes::std_stream::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::imap::{
    account::ImapAccount,
    mailbox::arg::{MailboxNameOptionalFlag, MailboxNoSelectFlag},
};

/// Remove IMAP flag(s) from message(s).
///
/// This command removes the specified flag(s) from message(s)
/// identified by the given sequence set.
#[derive(Debug, Parser)]
pub struct ImapFlagRemoveCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameOptionalFlag,
    #[command(flatten)]
    pub mailbox_no_select: MailboxNoSelectFlag,

    /// The sequence set of messages (e.g., "1", "1,2,3", "1:*").
    #[arg(name = "sequence_set", value_name = "SEQUENCE")]
    pub sequence_set: String,
    /// The flags to remove (e.g., "\\Seen", "\\Flagged").
    #[arg(short, long, required = true, num_args = 1..)]
    pub flag: Vec<String>,

    /// Use sequence numbers instead of UIDs.
    #[arg(long)]
    pub seq: bool,
}

impl ImapFlagRemoveCommand {
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
        let flags: Vec<Flag<'static>> = self
            .flag
            .iter()
            .map(|f| Flag::try_from(f.as_str()).map(|flag| flag.into_static()))
            .collect::<Result<_, _>>()?;

        let mut arg = None;
        let mut coroutine = ImapMessageStoreSilent::new(
            imap.context,
            sequence_set,
            StoreType::Remove,
            flags,
            !self.seq,
        );

        loop {
            match coroutine.resume(arg.take()) {
                ImapMessageStoreSilentResult::Io { input } => {
                    arg = Some(handle(&mut imap.stream, input)?)
                }
                ImapMessageStoreSilentResult::Ok { .. } => break,
                ImapMessageStoreSilentResult::Err { err, .. } => bail!(err),
            }
        }

        printer.out(Message::new("Flag(s) successfully removed"))
    }
}
