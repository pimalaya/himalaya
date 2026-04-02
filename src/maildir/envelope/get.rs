use std::fmt;

use anyhow::{bail, Result};
use clap::Parser;
use comfy_table::{Cell, Row, Table};
use io_fs::runtimes::std::handle;
use io_maildir::{coroutines::get_message::*, maildir::Maildir};
use mail_parser::Header;
use pimalaya_toolbox::terminal::printer::Printer;
use serde::Serialize;

use crate::maildir::{
    account::MaildirAccount,
    arg::{MaildirPathFlag, MessageIdArg},
};

/// Get a single MAILDIR envelope.
///
/// This command displays detailed envelope information for a specific
/// message, including all header fields like date, subject, from, to,
/// cc, bcc, reply-to, message-id, and in-reply-to.
#[derive(Debug, Parser)]
pub struct MaildirEnvelopeGetCommand {
    #[command(flatten)]
    pub maildir: MaildirPathFlag,
    #[command(flatten)]
    pub id: MessageIdArg,
}

impl MaildirEnvelopeGetCommand {
    pub fn execute(self, printer: &mut impl Printer, account: MaildirAccount) -> Result<()> {
        let maildir = match Maildir::try_from(self.maildir.inner.clone()) {
            Ok(maildir) => maildir,
            Err(_) => Maildir::try_from(account.backend.root.join(self.maildir.inner))?,
        };

        let mut arg = None;
        let mut coroutine = GetMaildirMessage::new(maildir, &self.id.inner);

        let message = loop {
            match coroutine.resume(arg.take()) {
                GetMaildirMessageResult::Io(io) => arg = Some(handle(io)?),
                GetMaildirMessageResult::Ok(msg) => break msg,
                GetMaildirMessageResult::Err(err) => bail!(err),
            };
        };

        let Some(message) = message.headers() else {
            bail!("Invalid MIME message at {}", message.path().display());
        };

        let table = EnvelopeTable {
            preset: account.table_preset,
            headers: message.headers(),
        };

        printer.out(table)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct EnvelopeTable<'a> {
    #[serde(skip)]
    pub preset: String,
    pub headers: &'a [Header<'a>],
}

impl fmt::Display for EnvelopeTable<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(&self.preset)
            .set_header(Row::from([Cell::new("HEADER"), Cell::new("VALUE")]));

        for header in self.headers {
            writeln!(f, "{}: {:?}", header.name.as_str(), header.value)?;
        }

        Ok(())
    }
}
