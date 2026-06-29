use std::{
    fmt,
    io::{Write, stdout},
};

use anyhow::{Result, bail};
use clap::Parser;
use mail_parser::{Addr, Address, HeaderValue, Message, MessageParser};
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::account::context::Account;
use crate::shared::{client::EmailClient, mailbox::arg::MailboxArg};

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

    #[command(flatten)]
    pub mailbox: MailboxArg,

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

        let mailbox = self.mailbox.resolve(account)?;
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
            let value = render_header_value(&header.value);
            writeln!(f, "{}: {value}", header.name.as_str())?;
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

/// Renders a parsed header value as decoded, human-readable text rather
/// than its `Debug` form.
fn render_header_value(value: &HeaderValue) -> String {
    match value {
        HeaderValue::Text(text) => text.to_string(),
        HeaderValue::TextList(list) => list
            .iter()
            .map(|text| text.as_ref())
            .collect::<Vec<_>>()
            .join(", "),
        HeaderValue::Address(address) => {
            let addrs: Vec<String> = match address {
                Address::List(list) => list.iter().map(format_addr).collect(),
                Address::Group(groups) => groups
                    .iter()
                    .flat_map(|group| group.addresses.iter())
                    .map(format_addr)
                    .collect(),
            };
            addrs.join(", ")
        }
        HeaderValue::DateTime(date) => date.to_rfc822(),
        HeaderValue::ContentType(ctype) => match ctype.subtype() {
            Some(subtype) => format!("{}/{subtype}", ctype.ctype()),
            None => ctype.ctype().to_owned(),
        },
        HeaderValue::Received(_) | HeaderValue::Empty => String::new(),
    }
}

/// Formats a single address as `Name <addr>`, or just the address when
/// it has no display name.
fn format_addr(addr: &Addr) -> String {
    let email = addr.address.as_deref().unwrap_or_default();
    match addr.name.as_deref() {
        Some(name) if !name.is_empty() => format!("{name} <{email}>"),
        _ => email.to_owned(),
    }
}
