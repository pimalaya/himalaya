use anyhow::{bail, Result};
use clap::Parser;
use io_imap::{
    coroutines::{expunge::*, select::*, store::*},
    types::flag::{Flag, StoreType},
};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::{config::ImapConfig, imap::mailbox::arg::MailboxNameOptionalFlag, imap::stream};

/// Delete messages from a mailbox.
///
/// This command marks messages as deleted and expunges them from the
/// mailbox. The messages are permanently removed.
#[derive(Debug, Parser)]
pub struct DeleteMessageCommand {
    #[command(flatten)]
    pub mailbox: MailboxNameOptionalFlag,

    /// The sequence set of messages (e.g., "1", "1,2,3", "1:*").
    #[arg(name = "sequence_set", value_name = "SEQUENCE")]
    pub sequence_set: String,

    /// Use sequence numbers instead of UIDs.
    #[arg(long)]
    pub seq: bool,
}

impl DeleteMessageCommand {
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

        // Parse sequence set
        let sequence_set = self.sequence_set.as_str().try_into()?;

        // STORE +FLAGS \Deleted
        let mut arg = None;
        let mut coroutine = ImapStoreSilent::new(
            context,
            sequence_set,
            StoreType::Add,
            vec![Flag::Deleted],
            !self.seq,
        );

        let context = loop {
            match coroutine.resume(arg.take()) {
                ImapStoreSilentResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                ImapStoreSilentResult::Ok { context, .. } => break context,
                ImapStoreSilentResult::Err { err, .. } => bail!(err),
            }
        };

        // EXPUNGE
        let mut arg = None;
        let mut coroutine = ImapExpunge::new(context);

        let expunged = loop {
            match coroutine.resume(arg.take()) {
                ImapExpungeResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                ImapExpungeResult::Ok { expunged, .. } => break expunged,
                ImapExpungeResult::Err { err, .. } => bail!(err),
            }
        };

        printer.out(Message::new(format!(
            "{} message(s) successfully deleted",
            expunged.len()
        )))
    }
}
