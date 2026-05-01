use std::fmt;

use anyhow::{bail, Result};
use clap::Parser;
use io_maildir::{
    coroutines::message_get::{MaildirMessageGet, MaildirMessageGetArg, MaildirMessageGetResult},
    maildir::Maildir,
    types::Message,
};
use pimalaya_toolbox::terminal::printer::Printer;
use serde::Serialize;

use crate::maildir::{
    account::MaildirAccount,
    arg::{MaildirPathFlag, MessageIdArg},
    runtime,
};

/// Read message content.
///
/// This command fetches a message and displays its text content.
/// By default it shows plain text content; use --html to show HTML.
#[derive(Debug, Parser)]
pub struct MaildirMessageReadCommand {
    #[command(flatten)]
    pub maildir: MaildirPathFlag,
    #[command(flatten)]
    pub id: MessageIdArg,

    /// Show HTML content instead of plain text.
    #[arg(long)]
    pub html: bool,
    /// Terminal width for text wrapping.
    #[arg(long, short, default_value = "80")]
    pub width: usize,
}

impl MaildirMessageReadCommand {
    pub fn execute(self, printer: &mut impl Printer, account: MaildirAccount) -> Result<()> {
        let maildir = match Maildir::try_from(self.maildir.inner.clone()) {
            Ok(maildir) => maildir,
            Err(_) => Maildir::try_from(account.backend.root.join(self.maildir.inner))?,
        };

        let mut coroutine = MaildirMessageGet::new(maildir, &self.id.inner);
        let mut arg = None;

        let message = loop {
            match coroutine.resume(arg.take()) {
                MaildirMessageGetResult::Ok(msg) => break msg,
                MaildirMessageGetResult::WantsDirRead(paths) => {
                    arg = Some(MaildirMessageGetArg::DirRead(runtime::dir_read(paths)?));
                }
                MaildirMessageGetResult::WantsFileRead(paths) => {
                    arg = Some(MaildirMessageGetArg::FileRead(runtime::file_read(paths)?));
                }
                MaildirMessageGetResult::Err(err) => bail!("{err}"),
            }
        };

        let path = message.path().to_owned();

        let Some(parsed) = message.parsed() else {
            bail!("Invalid MIME message at {}", path.display());
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
