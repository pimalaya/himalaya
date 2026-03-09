use anyhow::{bail, Result};
use clap::Parser;
use io_imap::{
    coroutines::{select::*, store::*},
    types::{
        flag::{Flag, StoreType},
        IntoStatic,
    },
};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::imap::{
    account::ImapAccount,
    mailbox::arg::{MailboxNameOptionalFlag, MailboxSelectFlag},
    stream,
};

/// Add IMAP flag(s) to message(s).
///
/// This command adds the given flags to messages identified by the
/// given sequence set.
#[derive(Debug, Parser)]
pub struct AddFlagsCommand {
    #[command(flatten)]
    pub mailbox: MailboxNameOptionalFlag,
    #[command(flatten)]
    pub select: MailboxSelectFlag,

    /// The sequence set of messages (e.g., "1", "1,2,3", "1:*").
    #[arg(value_name = "SEQUENCE")]
    pub sequence_set: String,
    /// The flags to add (e.g., "\\Seen", "\\Flagged").
    #[arg(short, long, required = true, num_args = 1..)]
    pub flag: Vec<String>,

    /// Use sequence numbers instead of UIDs.
    #[arg(long)]
    pub seq: bool,
}

impl AddFlagsCommand {
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
        let flags: Vec<Flag<'static>> = self
            .flag
            .iter()
            .map(|f| Flag::try_from(f.as_str()).map(|flag| flag.into_static()))
            .collect::<Result<_, _>>()?;

        let mut arg = None;
        let mut coroutine =
            ImapStoreSilent::new(context, sequence_set, StoreType::Add, flags, !self.seq);

        loop {
            match coroutine.resume(arg.take()) {
                ImapStoreSilentResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                ImapStoreSilentResult::Ok { .. } => break,
                ImapStoreSilentResult::Err { err, .. } => bail!(err),
            }
        }

        printer.out(Message::new("Flag(s) successfully added"))
    }
}
