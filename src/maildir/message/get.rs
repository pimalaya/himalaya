use std::fmt;

use anyhow::{Result, bail};
use clap::Parser;
use mail_parser::Message;
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::maildir::{
    arg::{MaildirPathFlag, MessageIdArg},
    client::MaildirClient,
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
    pub fn execute(self, printer: &mut impl Printer, client: &mut MaildirClient) -> Result<()> {
        let maildir = client.resolve_maildir(&self.maildir.inner)?;

        let msg = client.get(maildir, &self.id.inner)?;

        let path = msg.path().clone();

        let Some(parsed) = msg.headers() else {
            bail!("Invalid MIME message at {path}");
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
