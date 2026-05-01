use std::io::{stdin, BufRead, IsTerminal, Read, Write};

use anyhow::{bail, Result};
use clap::Parser;
use io_imap::{
    rfc3501::append::*,
    types::{
        core::Literal, extensions::binary::LiteralOrLiteral8, flag::Flag, mailbox::Mailbox,
        IntoStatic,
    },
};
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::imap::{account::ImapAccount, mailbox::arg::MailboxNameArg};

const READ_BUFFER_SIZE: usize = 16 * 1024;

/// Save a message to a mailbox.
///
/// This command appends a message to the specified mailbox. The
/// message is read from stdin in RFC 5322 format (raw email).
#[derive(Debug, Parser)]
pub struct ImapMessageSaveCommand {
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

impl ImapMessageSaveCommand {
    pub fn execute(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let mut imap = account.new_imap_session()?;
        let mailbox: Mailbox<'static> = self.mailbox.inner.try_into()?;
        let message = if !self.message.is_empty() || stdin().is_terminal() || printer.is_json() {
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

        let mut coroutine = ImapMessageAppend::new(imap.context, mailbox, flags, None, message);
        let mut buf = [0u8; READ_BUFFER_SIZE];
        let mut arg: Option<&[u8]> = None;

        loop {
            match coroutine.resume(arg.take()) {
                ImapMessageAppendResult::Ok { .. } => break,
                ImapMessageAppendResult::WantsRead => {
                    let n = imap.stream.read(&mut buf)?;
                    arg = Some(&buf[..n]);
                }
                ImapMessageAppendResult::WantsWrite(bytes) => {
                    imap.stream.write_all(&bytes)?;
                    arg = None;
                }
                ImapMessageAppendResult::Err { err, .. } => bail!("{err}"),
            }
        }

        printer.out(Message::new("Message successfully saved"))
    }
}
