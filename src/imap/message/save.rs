use anyhow::Result;
use clap::Parser;
use io_imap::{
    rfc3501::append::ImapMessageAppendOptions,
    types::{IntoStatic, flag::Flag, mailbox::Mailbox},
};
use pimalaya_cli::printer::{Message, Printer};

use crate::{
    imap::{client::ImapClient, mailbox::arg::MailboxNameArg},
    shared::message::arg::MessageArg,
};

/// Append a message to a mailbox (APPEND, RFC 3501).
///
/// Uploads a message into the given mailbox. The message can be passed
/// as a positional file path, an inline raw string, or piped via stdin
/// (see [`MessageArg`] for resolution order).
#[derive(Debug, Parser)]
pub struct ImapMessageSaveCommand {
    #[command(flatten)]
    pub mailbox: MailboxNameArg,

    /// The flags to add to the message.
    #[arg(short, long, num_args = 0..)]
    pub flag: Vec<String>,

    #[command(flatten)]
    pub message: MessageArg,
}

impl ImapMessageSaveCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        let mailbox: Mailbox<'static> = self.mailbox.inner.try_into()?;
        let message = self.message.parse()?;

        let flags: Vec<Flag<'static>> = self
            .flag
            .iter()
            .map(String::as_str)
            .map(|f| Flag::try_from(f).map(IntoStatic::into_static))
            .collect::<Result<_, _>>()?;

        client.append(
            mailbox,
            message.as_bytes(),
            ImapMessageAppendOptions {
                flags,
                date: None,
                non_sync: false,
            },
        )?;

        printer.out(Message::new("Message successfully saved"))
    }
}
