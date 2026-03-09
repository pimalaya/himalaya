use std::io::{stdin, Read};

use anyhow::{bail, Result};
use clap::Parser;
use io_imap::{
    coroutines::{
        append::*,
        select::{ImapSelect, ImapSelectResult},
    },
    types::{core::Literal, extensions::binary::LiteralOrLiteral8, mailbox::Mailbox},
};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::imap::{account::ImapAccount, stream};

/// Save a message to a mailbox.
///
/// This command appends a message to the specified mailbox. The
/// message is read from stdin in RFC 5322 format (raw email).
#[derive(Debug, Parser)]
pub struct SaveMessageCommand {
    /// The mailbox to save the message to.
    #[arg(name = "mailbox", value_name = "MAILBOX")]
    pub mailbox: String,

    /// Select the given mailbox before saving message into it.
    ///
    /// This argument can be omitted when stateful IMAP sessions are
    /// used, for example with:
    ///
    /// https://github.com/pimalaya/sirup
    #[arg(short, long, default_value_t)]
    pub select: bool,
}

impl SaveMessageCommand {
    pub fn exec(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let (mut context, mut stream) = stream::connect(account.backend)?;

        // Read message from stdin
        let mut message = Vec::new();
        stdin().read_to_end(&mut message)?;

        if message.is_empty() {
            bail!("No message provided on stdin");
        }

        let mailbox: Mailbox<'static> = self.mailbox.try_into()?;
        let literal = Literal::try_from(message)?;
        let message = LiteralOrLiteral8::Literal(literal);

        if self.select {
            let mut arg = None;
            let mut coroutine = ImapSelect::new(context, mailbox.clone());

            context = loop {
                match coroutine.resume(arg.take()) {
                    ImapSelectResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                    ImapSelectResult::Ok { context, .. } => break context,
                    ImapSelectResult::Err { err, .. } => bail!(err),
                }
            };
        }

        let mut arg = None;
        let mut coroutine = ImapAppend::new(context, mailbox, vec![], None, message);

        loop {
            match coroutine.resume(arg.take()) {
                ImapAppendResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                ImapAppendResult::Ok { .. } => break,
                ImapAppendResult::Err { err, .. } => bail!(err),
            }
        }

        printer.out(Message::new("Message successfully saved"))
    }
}
