use std::io::{stdin, BufRead, IsTerminal};

use anyhow::{bail, Result};
use clap::Parser;
use io_imap::{
    coroutines::append::*,
    types::{
        core::Literal, extensions::binary::LiteralOrLiteral8, flag::Flag, mailbox::Mailbox,
        IntoStatic,
    },
};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::imap::{account::ImapAccount, mailbox::arg::MailboxNameArg, stream};

/// Save a message to a mailbox.
///
/// This command appends a message to the specified mailbox. The
/// message is read from stdin in RFC 5322 format (raw email).
#[derive(Debug, Parser)]
pub struct SaveMessageCommand {
    #[command(flatten)]
    pub mailbox: MailboxNameArg,

    /// The flags to add to the message.
    #[arg(short, long, num_args = 0..)]
    pub flag: Vec<String>,

    /// The raw message, including headers and body.
    #[arg(trailing_var_arg = true)]
    #[arg(name = "message", value_name = "MESSAGE")]
    pub message: Vec<String>,
}

impl SaveMessageCommand {
    pub fn exec(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let (context, mut stream) = stream::connect(account.backend)?;

        let mailbox: Mailbox<'static> = self.mailbox.name.try_into()?;

        let message = if stdin().is_terminal() || printer.is_json() {
            self.message
                .join(" ")
                .replace('\r', "")
                .replace('\n', "\r\n")
        } else {
            stdin()
                .lock()
                .lines()
                .map_while(Result::ok)
                .collect::<Vec<String>>()
                .join("\r\n")
        };
        let message = Literal::try_from(message)?;
        let message = LiteralOrLiteral8::Literal(message);

        let flags: Vec<_> = self
            .flag
            .iter()
            .map(String::as_str)
            .map(|f| Flag::try_from(f).map(IntoStatic::into_static))
            .collect::<Result<_, _>>()?;

        let mut arg = None;
        let mut coroutine = ImapAppend::new(context, mailbox, flags, None, message);

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
