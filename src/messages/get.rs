use std::{
    fmt,
    io::{stdout, Write},
};

use anyhow::{bail, Result};
use clap::Parser;
use mail_parser::{Message, MessageParser};
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::{
    cli::BackendArg,
    config::{AccountConfig, Config},
    messages::fetch::fetch_raw,
};

/// Get a message from the active account.
///
/// By default the message is parsed and rendered as headers + text
/// bodies. Pass `--raw` to dump the original RFC 5322 bytes to stdout
/// instead, or use the global `--json` flag to emit the parsed message
/// as JSON.
#[derive(Debug, Parser)]
pub struct MessagesGetCommand {
    /// Identifier of the message (IMAP UID, JMAP email id, or Maildir
    /// filename id).
    #[arg(value_name = "ID")]
    pub id: String,

    /// Mailbox name or path (IMAP mailbox / Maildir path). Ignored for
    /// JMAP, which addresses messages by id directly.
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

impl MessagesGetCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        config: Config,
        account_config: AccountConfig,
        backend: BackendArg,
    ) -> Result<()> {
        if self.raw && printer.is_json() {
            bail!("`--raw` and `--json` cannot be combined");
        }

        let raw = fetch_raw(&config, &account_config, backend, &self.mailbox, &self.id)?;

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
