//! # Maildir message copy
//!
//! The `maildir message copy` command, copying messages into another
//! Maildir.

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::{
    maildir::{
        arg::{MaildirPathFlag, MaildirSubdirArg, MessageIdsArg, TargetMaildirPathFlag},
        client::MaildirClient,
    },
    shared::receipt::IdsReceipt,
};

/// Copy Maildir message(s) to another folder.
///
/// Writes a copy of each message file identified by the given id(s)
/// into the target folder, printing the ids the copies got. Every id is
/// looked up first: an unknown one refuses the whole set, nothing
/// copied.
#[derive(Debug, Parser)]
pub struct MaildirMessageCopyCommand {
    #[command(flatten)]
    pub ids: MessageIdsArg,
    #[command(flatten)]
    pub source: MaildirPathFlag,
    #[command(flatten)]
    pub target: TargetMaildirPathFlag,
    /// Copy the message into a different subdirectory; a copy into `new`
    /// carries no flags, a Maildir keeping none there.
    #[arg(long, short, value_name = "DIR", value_enum)]
    pub subdir: Option<MaildirSubdirArg>,
}

impl MaildirMessageCopyCommand {
    /// Copies the messages into the destination Maildir.
    pub fn execute(self, printer: &mut impl Printer, client: &mut MaildirClient) -> Result<()> {
        let source = client.resolve_maildir(&self.source.inner)?;
        let target = client.resolve_maildir(&self.target.inner)?;
        let ids: Vec<&str> = self.ids.inner.iter().map(String::as_str).collect();
        let found = client.found_all(&source, &ids)?;

        let mut ids = Vec::with_capacity(found.len());
        for id in found {
            ids.push(client.copy(
                id,
                source.clone(),
                target.clone(),
                self.subdir.clone().map(Into::into),
            )?);
        }

        printer.out(IdsReceipt {
            ids,
            done: "Message(s) successfully copied",
        })
    }
}
