use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::maildir::{
    arg::{MaildirPathFlag, MaildirSubdirArg, MessageIdsArg, TargetMaildirPathFlag},
    client::MaildirClient,
};

/// Move Maildir message to the given mailbox.
///
/// This command copies message(s) identified by the given sequence
/// set from the source mailbox to the destination mailbox.
#[derive(Debug, Parser)]
pub struct MaildirMessageMoveCommand {
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

        printer.out(Message::new("Message(s) successfully copied"))
    }
}
