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

        let mut coroutine = MaildirMessageGet::new(maildir, &self.id.inner);
        let mut arg = None;

        let msg = loop {
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

        let path = msg.path().to_owned();

        let Some(parsed) = msg.headers() else {
            bail!("Invalid MIME message at {}", path.display());
        };

        printer.out(MessageView(parsed))
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
