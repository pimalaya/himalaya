use anyhow::Result;
use clap::Parser;
use io_maildir::maildir::Maildir;
use pimalaya_cli::printer::{Message, Printer};

use crate::maildir::{
    account::MaildirAccount,
    arg::{MaildirPathFlag, MaildirSubdirArg, MessageIdsArg, TargetMaildirPathFlag},
};

/// Copy Maildir message to the given mailbox.
///
/// This command copies message(s) identified by the given sequence
/// set from the source mailbox to the destination mailbox.
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
    pub fn execute(self, printer: &mut impl Printer, account: MaildirAccount) -> Result<()> {
        let source = match Maildir::try_from(self.source.inner.clone()) {
            Ok(maildir) => maildir,
            Err(_) => Maildir::try_from(account.backend.root.join(&self.source.inner))?,
        };

        let target = match Maildir::try_from(self.target.inner.clone()) {
            Ok(maildir) => maildir,
            Err(_) => Maildir::try_from(account.backend.root.join(&self.target.inner))?,
        };

        let client = account.new_maildir_client();

        for id in self.ids.inner {
            client.copy(
                id,
                source.clone(),
                target.clone(),
                self.subdir.clone().map(Into::into),
            )?;
        }

        printer.out(Message::new("Message(s) successfully copied"))
    }
}
