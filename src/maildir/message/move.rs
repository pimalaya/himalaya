//! # Maildir message move
//!
//! The `maildir message move` command, moving messages into another
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

/// Move Maildir message(s) to another folder.
///
/// Relocates each message file identified by the given id(s) from the
/// source folder into the target folder, printing the ids the messages
/// got there. Every id is looked up first: an unknown one refuses the
/// whole set, nothing moved. A move within one folder without
/// `--subdir` changes nothing, the ids kept.
#[derive(Debug, Parser)]
pub struct MaildirMessageMoveCommand {
    #[command(flatten)]
    pub ids: MessageIdsArg,
    #[command(flatten)]
    pub source: MaildirPathFlag,
    #[command(flatten)]
    pub target: TargetMaildirPathFlag,
    /// Move the message into a different subdirectory; a message moved
    /// into `new` loses its flags, a Maildir keeping none there.
    #[arg(long, short, value_name = "DIR", value_enum)]
    pub subdir: Option<MaildirSubdirArg>,
}

impl MaildirMessageMoveCommand {
    /// Moves the messages into the destination Maildir.
    pub fn execute(self, printer: &mut impl Printer, client: &mut MaildirClient) -> Result<()> {
        let source = client.resolve_maildir(&self.source.inner)?;
        let target = client.resolve_maildir(&self.target.inner)?;
        let ids: Vec<&str> = self.ids.inner.iter().map(String::as_str).collect();
        let found = client.found_all(&source, &ids)?;

        if source.path() == target.path() && self.subdir.is_none() {
            let ids = found.into_iter().map(String::from).collect();
            return printer.out(IdsReceipt {
                ids,
                done: "Message(s) successfully moved",
            });
        }

        let mut ids = Vec::with_capacity(found.len());
        for id in found {
            ids.push(client.r#move(
                id,
                source.clone(),
                target.clone(),
                self.subdir.clone().map(Into::into),
            )?);
        }

        printer.out(IdsReceipt {
            ids,
            done: "Message(s) successfully moved",
        })
    }
}
