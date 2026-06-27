use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::maildir::{
    arg::{MaildirPathFlag, MaildirSubdirArg, MessageIdsArg, TargetMaildirPathFlag},
    client::MaildirClient,
};

/// Move Maildir message(s) to another folder.
///
/// Relocates each message file identified by the given id(s) from the
/// source folder into the target folder.
#[derive(Debug, Parser)]
pub struct MaildirMessageMoveCommand {
    #[command(flatten)]
    pub ids: MessageIdsArg,
    #[command(flatten)]
    pub source: MaildirPathFlag,
    #[command(flatten)]
    pub target: TargetMaildirPathFlag,

    /// Move the message into a different subdirectory.
    #[arg(long, short, value_name = "DIR", value_enum)]
    pub subdir: Option<MaildirSubdirArg>,
}

impl MaildirMessageMoveCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut MaildirClient) -> Result<()> {
        let source = client.resolve_maildir(&self.source.inner)?;
        let target = client.resolve_maildir(&self.target.inner)?;

        for id in self.ids.inner {
            client.r#move(
                id,
                source.clone(),
                target.clone(),
                self.subdir.clone().map(Into::into),
            )?;
        }

        printer.out(Message::new("Message(s) successfully moved"))
    }
}
