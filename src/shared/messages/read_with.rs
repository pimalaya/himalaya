use std::io::{stdout, Write};

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::shared::{client::EmailClient, messages::runner};

/// Read a message by delegating to a user-defined reader.
///
/// Fetches the source and pipes it on stdin to the named (or
/// default) reader. The reader's stdout is forwarded to the
/// terminal — zero bytes is fine (the reader may have spawned its
/// own UI), non-empty bytes are written as-is.
#[derive(Debug, Parser)]
pub struct MessageReadWithCommand {
    /// Identifier of the message.
    #[arg(value_name = "ID")]
    pub id: String,

    /// Mailbox the message lives in. Ignored for JMAP.
    #[arg(
        long = "mailbox",
        short = 'm',
        value_name = "NAME",
        default_value = "Inbox"
    )]
    pub mailbox: String,

    /// Name of an entry in `[message.reader.*]`. Optional — when
    /// omitted, the reader flagged `default = true` is used.
    #[arg(value_name = "NAME", conflicts_with = "command")]
    pub name: Option<String>,

    /// Ad-hoc shell command, mutually exclusive with `<name>`.
    #[arg(long, value_name = "SHELL")]
    pub command: Option<String>,
}

impl MessageReadWithCommand {
    pub fn execute(self, _printer: &mut impl Printer, mut client: EmailClient) -> Result<()> {
        let source = client.get_message(&self.mailbox, &self.id)?;

        let command = match self.command.as_deref() {
            Some(cmd) => cmd.to_owned(),
            None => {
                runner::resolve_reader(&client.account.reader, self.name.as_deref())?.to_owned()
            }
        };

        let bytes = runner::run(&command, &source)?;

        if !bytes.is_empty() {
            let mut out = stdout().lock();
            out.write_all(&bytes)?;
        }

        Ok(())
    }
}
