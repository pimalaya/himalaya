use std::{fmt, num::NonZeroU32};

use anyhow::{bail, Result};
use clap::Parser;
use io_imap::{
    coroutines::{fetch::*, select::*},
    types::fetch::{MacroOrMessageDataItemNames, MessageDataItem, MessageDataItemName},
};
use io_stream::runtimes::std::handle;
use mail_parser::MessageParser;
use pimalaya_toolbox::terminal::printer::Printer;
use serde::Serialize;

use crate::imap::{account::ImapAccount, mailbox::arg::MailboxNameOptionalFlag, stream};

/// Read message content.
///
/// This command fetches a message and displays its text content.
/// By default it shows plain text content; use --html to show HTML.
#[derive(Debug, Parser)]
pub struct ReadMessageCommand {
    #[command(flatten)]
    pub mailbox: MailboxNameOptionalFlag,

    /// The message UID (or sequence number with --seq).
    #[arg(name = "id", value_name = "ID")]
    pub id: u32,

    /// Use sequence numbers instead of UIDs.
    #[arg(long)]
    pub seq: bool,

    /// Show HTML content instead of plain text.
    #[arg(long)]
    pub html: bool,

    /// Terminal width for text wrapping.
    #[arg(long, short = 'w', default_value = "80")]
    pub width: usize,
}

impl ReadMessageCommand {
    pub fn exec(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let (context, mut stream) = stream::connect(account.backend)?;

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

        // FETCH with BODY.PEEK[] to avoid marking as read
        let id = NonZeroU32::new(self.id).ok_or_else(|| anyhow::anyhow!("ID must be non-zero"))?;

        let item_names =
            MacroOrMessageDataItemNames::MessageDataItemNames(vec![MessageDataItemName::BodyExt {
                section: None,
                partial: None,
                peek: true,
            }]);

        let mut arg = None;
        let mut coroutine = ImapFetchFirst::new(context, id, item_names, !self.seq);

        let items = loop {
            match coroutine.resume(arg.take()) {
                ImapFetchFirstResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                ImapFetchFirstResult::Ok { items, .. } => break items,
                ImapFetchFirstResult::Err { err, .. } => bail!(err),
            }
        };

        // Extract raw message bytes
        let mut raw_message: Option<Vec<u8>> = None;
        for item in items.into_iter() {
            if let MessageDataItem::BodyExt { data, .. } = item {
                if let Some(data) = data.0 {
                    raw_message = Some(data.as_ref().to_vec());
                }
            }
        }

        let raw = raw_message.ok_or_else(|| anyhow::anyhow!("No message data returned"))?;

        // Parse message using mail-parser
        let message = MessageParser::default()
            .parse(&raw)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse message"))?;

        let content = if self.html {
            // Get HTML content
            message
                .body_html(0)
                .map(|s| s.to_string())
                .ok_or_else(|| anyhow::anyhow!("No HTML content found"))?
        } else {
            // Get plain text, or convert HTML to text
            if let Some(text) = message.body_text(0) {
                text.to_string()
            } else if let Some(html) = message.body_html(0) {
                // Convert HTML to text
                html2text::from_read(html.as_bytes(), self.width)
            } else {
                bail!("No text or HTML content found");
            }
        };

        let output = MessageContent { content };

        printer.out(output)?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct MessageContent {
    pub content: String,
}

impl fmt::Display for MessageContent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f)?;
        write!(f, "{}", self.content)?;
        if !self.content.ends_with('\n') {
            writeln!(f)?;
        }
        Ok(())
    }
}

impl Serialize for MessageContent {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.content.serialize(serializer)
    }
}
