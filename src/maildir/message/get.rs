use std::fmt;

use anyhow::{bail, Result};
use clap::Parser;
use io_fs::runtimes::std::handle;
use io_maildir::{coroutines::get_message::*, maildir::Maildir, types::Message};
use pimalaya_toolbox::terminal::printer::Printer;
use serde::Serialize;

use crate::maildir::{
    account::MaildirAccount,
    arg::{MaildirPathFlag, MessageIdArg},
};

/// Get Maildir message to the given mailbox.
///
/// This command copies message(s) identified by the given sequence
/// set from the source mailbox to the destination mailbox.
#[derive(Debug, Parser)]
pub struct MaildirMessageGetCommand {
    #[command(flatten)]
    pub maildir: MaildirPathFlag,
    #[command(flatten)]
    pub id: MessageIdArg,
}

impl MaildirMessageGetCommand {
    pub fn execute(self, printer: &mut impl Printer, account: MaildirAccount) -> Result<()> {
        let maildir = match Maildir::try_from(self.maildir.inner.clone()) {
            Ok(maildir) => maildir,
            Err(_) => Maildir::try_from(account.backend.root.join(self.maildir.inner))?,
        };

        let mut arg = None;
        let mut coroutine = GetMaildirMessage::new(maildir, &self.id.inner);

        let msg = loop {
            match coroutine.resume(arg.take()) {
                GetMaildirMessageResult::Io(io) => arg = Some(handle(io)?),
                GetMaildirMessageResult::Ok(msg) => break msg,
                GetMaildirMessageResult::Err(err) => bail!(err),
            };
        };

        let Some(msg) = msg.headers() else {
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
        for header in self.0.headers() {
            writeln!(f, "{}: {:?}", header.name(), header.value())?;
        }

        Ok(())
    }
}
