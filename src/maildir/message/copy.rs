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
/// into the target folder.
#[derive(Debug, Parser)]
pub struct MaildirMessageCopyCommand {
    #[command(flatten)]
    pub ids: MessageIdsArg,
    #[command(flatten)]
    pub source: MaildirPathFlag,
    #[command(flatten)]
    pub target: TargetMaildirPathFlag,
    /// Copy the message into a different subdirectory.
    #[arg(long, short, value_name = "DIR", value_enum)]
    pub subdir: Option<MaildirSubdirArg>,
}

impl MaildirMessageCopyCommand {
    /// Copies the messages into the destination Maildir.
    pub fn execute(self, printer: &mut impl Printer, client: &mut MaildirClient) -> Result<()> {
        let source = client.resolve_maildir(&self.source.inner)?;
        let target = client.resolve_maildir(&self.target.inner)?;

        for id in self.ids.inner.clone() {
            client.copy(
                id,
                source.clone(),
                target.clone(),
                self.subdir.clone().map(Into::into),
            )?;
        }

        printer.out(IdsReceipt {
            ids: self.ids.inner,
            done: "Message(s) successfully copied",
        })
    }
}
