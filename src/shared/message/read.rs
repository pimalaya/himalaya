use std::{
    fmt,
    io::{Write, stdout},
};

use anyhow::{Result, bail};
use clap::Parser;
use mail_parser::{Message, MessageParser};
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::account::context::Account;
use crate::shared::client::EmailClient;

/// Read a message from the active account (built-in flag reader).
///
/// Fetches the message and renders headers + text bodies. Pass
/// `--raw` to dump the original RFC 5322 bytes to stdout instead,
/// or `--json` to emit the parsed message as JSON. For a custom
/// pretty-printer (`mml interpret`, w3m, your own viewer), pipe the
/// `--raw` output into the renderer of your choice.
#[derive(Debug, Parser)]
pub struct MessageReadCommand {
    /// Identifier of the message (IMAP UID, JMAP email id, or Maildir
    /// filename id).
    #[arg(value_name = "ID")]
    pub id: String,

    /// Mailbox name or alias (IMAP mailbox / Maildir path). Ignored
    /// for JMAP, which addresses messages by id directly.
    #[arg(
        long = "mailbox",
        short = 'm',
        value_name = "NAME",
        default_value = "Inbox"
    )]
    pub mailbox: String,

    /// Write the raw RFC 5322 bytes to stdout. Mutually exclusive with
    /// the global `--json` flag.
    #[arg(long)]
    pub raw: bool,
}

impl MessageReadCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        if self.raw && printer.is_json() {
            bail!("`--raw` and `--json` cannot be combined");
        }

        let mailbox = account.resolve_mailbox(&self.mailbox).to_owned();
        let raw = client.get_message(&mailbox, &self.id)?;

        if self.raw {
            let mut out = stdout().lock();
            out.write_all(&raw)?;
            return Ok(());
        }

        let Some(parsed) = MessageParser::new().parse(&raw) else {
            bail!("Failed to parse RFC 5322 message");
        };

        printer.out(MessageView(parsed.into_owned()))
    }
}

/// Parsed message rendered as headers plus text bodies, or as JSON.
#[derive(Serialize)]
#[serde(transparent)]
pub struct MessageView(Message<'static>);

impl fmt::Display for MessageView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for header in self.0.headers() {
            writeln!(f, "{}: {:?}", header.name.as_str(), header.value)?;
        }

        writeln!(f)?;

        for (i, part) in self.0.text_bodies().enumerate() {
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
