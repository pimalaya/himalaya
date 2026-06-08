use std::fmt;

use anyhow::{Result, bail};
use clap::Parser;
use mail_parser::{Message, MessageParser};
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::m2dir::{
    arg::{M2dirNameFlag, MessageIdArg},
    client::M2dirClient,
};

/// Read m2dir message content.
///
/// Fetches a message and displays its text content. By default it
/// shows plain text content; use `--html` to show HTML.
#[derive(Debug, Parser)]
pub struct M2dirMessageReadCommand {
    #[command(flatten)]
    pub m2dir: M2dirNameFlag,
    #[command(flatten)]
    pub id: MessageIdArg,

    /// Show HTML content instead of plain text.
    #[arg(long)]
    pub html: bool,
    /// Terminal width for text wrapping.
    #[arg(long, short, default_value = "80")]
    pub width: usize,
}

impl M2dirMessageReadCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut M2dirClient) -> Result<()> {
        let store = client.open_store()?;
        let path = store.resolve_folder_path(&self.m2dir.inner)?;
        let m2dir = client.open_m2dir(path)?;
        let (entry, bytes) = client.get(m2dir, &self.id.inner)?;

        let Some(parsed) = MessageParser::new().parse(&bytes) else {
            let path = entry.path();
            bail!("Invalid MIME message at {path}");
        };

        if self.html {
            printer.out(MessageHtmlView { message: parsed })
        } else {
            printer.out(MessagePlainView { message: parsed })
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
