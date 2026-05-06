use std::io::{stdin, BufRead, IsTerminal};

use anyhow::Result;
use clap::Parser;
use io_imap::types::{
    core::Literal, extensions::binary::LiteralOrLiteral8, flag::Flag, mailbox::Mailbox, IntoStatic,
};
use pimalaya_cli::printer::{Message, Printer};

use crate::imap::{client::ImapClient, mailbox::arg::MailboxNameArg};

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
    pub fn execute(self, printer: &mut impl Printer, mut client: ImapClient) -> Result<()> {
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

        client.append(mailbox, flags, None, message)?;

        printer.out(Message::new("Message successfully saved"))
    }
}
