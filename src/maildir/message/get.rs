use std::{fmt, path::PathBuf};

use anyhow::{bail, Result};
use clap::Parser;
use io_fs::runtimes::std::handle;
use io_maildir::{
    coroutines::get_message::*,
    maildir::Maildir,
    types::{Message, PartType},
};
use pimalaya_toolbox::terminal::printer::Printer;
use serde::Serialize;

use crate::maildir::account::MaildirAccount;

/// Get Maildir message to the given mailbox.
///
/// This command copies message(s) identified by the given sequence
/// set from the source mailbox to the destination mailbox.
#[derive(Debug, Parser)]
pub struct GetMessageCommand {
    /// Path to the Maildir containing the message looked for.
    #[arg(long, short, value_name = "PATH")]
    #[arg(default_value = "Inbox")]
    pub maildir: PathBuf,

    /// Id of message to get.
    #[arg(value_name = "ID")]
    pub id: String,
}

impl GetMessageCommand {
    pub fn execute(self, printer: &mut impl Printer, account: MaildirAccount) -> Result<()> {
        let maildir = match Maildir::try_from(self.maildir.clone()) {
            Ok(maildir) => maildir,
            Err(_) => Maildir::try_from(account.backend.root.join(self.maildir))?,
        };

        let mut arg = None;
        let mut coroutine = GetMaildirMessage::new(maildir, &self.id);

        let msg = loop {
            match coroutine.resume(arg.take()) {
                GetMaildirMessageResult::Io(io) => arg = Some(handle(io)?),
                GetMaildirMessageResult::Ok(msg) => break msg,
                GetMaildirMessageResult::Err(err) => bail!(err),
            };
        };

        let Some(msg) = msg.parsed() else {
            bail!("Invalid MIME message at {}", msg.path().display());
        };

        printer.out(MessageView(msg))
    }
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct MessageView<'a>(Message<'a>);

impl fmt::Display for MessageView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts_len = self.0.parts.len();
        for (i, p) in self.0.parts.iter().enumerate() {
            writeln!(f, "---")?;
            writeln!(f, "Part {}/{parts_len}:", i + 1)?;
            writeln!(f)?;

            for h in p.headers() {
                writeln!(f, "{}: {:?}", h.name.as_str(), h.value)?;
            }

            writeln!(f)?;

            match &p.body {
                PartType::Text(p) => writeln!(f, "{p}")?,
                PartType::Html(p) => writeln!(f, "{p}")?,
                PartType::Binary(p) => writeln!(f, "({} bytes)", p.len())?,
                PartType::InlineBinary(p) => writeln!(f, "({} inline bytes)", p.len())?,
                PartType::Multipart(_) => continue,
                PartType::Message(m) => write!(f, "{}", MessageView(m.clone()))?,
            }
        }

        Ok(())
    }
}
