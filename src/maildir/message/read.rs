use std::{fmt, path::PathBuf};

use anyhow::{bail, Result};
use clap::Parser;
use io_fs::runtimes::std::handle;
use io_maildir::{coroutines::get_message::*, maildir::Maildir, types::Message};
use pimalaya_toolbox::terminal::printer::Printer;
use serde::Serialize;

use crate::maildir::account::MaildirAccount;

/// Read message content.
///
/// This command fetches a message and displays its text content.
/// By default it shows plain text content; use --html to show HTML.
#[derive(Debug, Parser)]
pub struct ReadMessageCommand {
    /// Path to the Maildir containing the message looked for.
    #[arg(long, short, value_name = "PATH")]
    #[arg(default_value = "Inbox")]
    pub maildir: PathBuf,

    /// Id of message to read.
    #[arg()]
    pub id: String,

    /// Show HTML content instead of plain text.
    #[arg(long)]
    pub html: bool,

    /// Terminal width for text wrapping.
    #[arg(long, short, default_value = "80")]
    pub width: usize,
}

impl ReadMessageCommand {
    pub fn execute(self, printer: &mut impl Printer, account: MaildirAccount) -> Result<()> {
        let maildir = match Maildir::try_from(self.maildir.clone()) {
            Ok(maildir) => maildir,
            Err(_) => Maildir::try_from(account.backend.root.join(self.maildir))?,
        };

        let mut arg = None;
        let mut coroutine = GetMaildirMessage::new(maildir, &self.id);

        let message = loop {
            match coroutine.resume(arg.take()) {
                GetMaildirMessageResult::Io(io) => arg = Some(handle(io)?),
                GetMaildirMessageResult::Ok(msg) => break msg,
                GetMaildirMessageResult::Err(err) => bail!(err),
            };
        };

        let Some(message) = message.parsed() else {
            bail!("Invalid MIME message at {}", message.path().display());
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
