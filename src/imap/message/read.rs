use std::{
    fmt,
    io::{Read, Write},
    num::NonZeroU32,
};

use anyhow::{bail, Result};
use clap::Parser;
use io_imap::{
    rfc3501::{fetch::*, select::*},
    types::fetch::{MacroOrMessageDataItemNames, MessageDataItem, MessageDataItemName},
};
use mail_parser::{Message, MessageParser};
use pimalaya_toolbox::terminal::printer::Printer;
use serde::Serialize;

use crate::imap::{
    account::ImapAccount,
    mailbox::arg::{MailboxNameOptionalFlag, MailboxNoSelectFlag},
};

const READ_BUFFER_SIZE: usize = 16 * 1024;

/// Read message content.
///
/// This command fetches a message and displays its text content.
/// By default it shows plain text content; use --html to show HTML.
#[derive(Debug, Parser)]
pub struct ImapMessageReadCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameOptionalFlag,
    #[command(flatten)]
    pub mailbox_no_select: MailboxNoSelectFlag,

    /// The message UID (or sequence number with --seq).
    #[arg(name = "id", value_name = "ID")]
    pub id: u32,

    /// Use sequence numbers instead of UIDs.
    #[arg(long)]
    pub seq: bool,

    /// Show HTML content instead of plain text.
    #[arg(long)]
    pub html: bool,
}

impl ImapMessageReadCommand {
    pub fn execute(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let mut imap = account.new_imap_session()?;
        let mailbox = self.mailbox_name.inner.try_into()?;

        let mut buf = [0u8; READ_BUFFER_SIZE];

        if !self.mailbox_no_select.inner {
            let mut coroutine = ImapMailboxSelect::new(imap.context, mailbox);
            let mut arg: Option<&[u8]> = None;

            imap.context = loop {
                match coroutine.resume(arg.take()) {
                    ImapMailboxSelectResult::Ok { context, .. } => break context,
                    ImapMailboxSelectResult::WantsRead => {
                        let n = imap.stream.read(&mut buf)?;
                        arg = Some(&buf[..n]);
                    }
                    ImapMailboxSelectResult::WantsWrite(bytes) => {
                        imap.stream.write_all(&bytes)?;
                        arg = None;
                    }
                    ImapMailboxSelectResult::Err { err, .. } => bail!("{err}"),
                }
            };
        }

        let Some(id) = NonZeroU32::new(self.id) else {
            bail!("ID must be non-zero");
        };

        let item_names =
            MacroOrMessageDataItemNames::MessageDataItemNames(vec![MessageDataItemName::BodyExt {
                section: None,
                partial: None,
                peek: true,
            }]);

        let mut coroutine = ImapMessageFetchFirst::new(imap.context, id, item_names, !self.seq);
        let mut arg: Option<&[u8]> = None;

        let items = loop {
            match coroutine.resume(arg.take()) {
                ImapMessageFetchFirstResult::Ok { items, .. } => break items,
                ImapMessageFetchFirstResult::WantsRead => {
                    let n = imap.stream.read(&mut buf)?;
                    arg = Some(&buf[..n]);
                }
                ImapMessageFetchFirstResult::WantsWrite(bytes) => {
                    imap.stream.write_all(&bytes)?;
                    arg = None;
                }
                ImapMessageFetchFirstResult::Err { err, .. } => bail!("{err}"),
            }
        };

        let mut raw_message: Option<Vec<u8>> = None;

        for item in items.into_iter() {
            if let MessageDataItem::BodyExt { data, .. } = item {
                if let Some(data) = data.0 {
                    raw_message = Some(data.as_ref().to_vec());
                }
            }
        }

        let Some(raw) = raw_message else {
            bail!("Read message `{}` error: no message data returned", self.id);
        };

        let Some(message) = MessageParser::new().parse(&raw) else {
            bail!(
                "Read message `{}` error: failed to parse MIME message",
                self.id
            );
        };

        if self.html {
            printer.out(MessageHtmlView { message })
        } else {
            printer.out(MessagePlainView { message })
        }
    }
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct MessagePlainView<'a> {
    message: Message<'a>,
}

impl fmt::Display for MessagePlainView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, part) in self.message.text_bodies().enumerate() {
            if i > 0 {
                writeln!(f)?;
                writeln!(f)?;
            }

            if let Some(contents) = part.text_contents() {
                write!(f, "{}", contents.trim_end())?;
            }
        }

        Ok(())
    }
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct MessageHtmlView<'a> {
    message: Message<'a>,
}

impl fmt::Display for MessageHtmlView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, part) in self.message.html_bodies().enumerate() {
            if i > 0 {
                writeln!(f)?;
                writeln!(f)?;
            }

            if let Some(contents) = part.text_contents() {
                write!(f, "{}", contents.trim_end())?;
            }
        }

        Ok(())
    }
}
