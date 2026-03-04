use anyhow::{bail, Result};
use clap::Parser;
use io_imap::{
    coroutines::{copy::*, select::*},
    types::mailbox::Mailbox,
};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::{config::ImapConfig, imap::mailbox::arg::name::MailboxNameOptionalFlag, imap::stream};

/// Copy messages to another mailbox.
///
/// This command copies messages identified by the given sequence set
/// from the source mailbox to the destination mailbox.
#[derive(Debug, Parser)]
pub struct CopyMessageCommand {
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

impl CopyMessageCommand {
    pub fn execute(self, printer: &mut impl Printer, config: ImapConfig) -> Result<()> {
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

        // COPY
        let mut arg = None;
        let mut coroutine = ImapCopy::new(context, sequence_set, destination, !self.seq);

        loop {
            match coroutine.resume(arg.take()) {
                ImapCopyResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                ImapCopyResult::Ok { .. } => break,
                ImapCopyResult::Err { err, .. } => bail!(err),
            }
        }

        printer.out(Message::new("Message(s) successfully copied"))
    }
}
