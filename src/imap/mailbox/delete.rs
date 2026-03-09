use anyhow::{bail, Result};
use clap::Parser;
use io_imap::coroutines::delete::*;
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::imap::{account::ImapAccount, mailbox::arg::MailboxNameArg, stream};

/// Delete the given mailbox.
///
/// All emails from the given mailbox are definitely deleted. The
/// mailbox is also deleted after execution of the command.
#[derive(Debug, Parser)]
pub struct DeleteMailboxCommand {
    #[command(flatten)]
    pub mailbox: MailboxNameArg,
}

impl DeleteMailboxCommand {
    pub fn exec(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let (context, mut stream) = stream::connect(account.backend)?;

        let mailbox = self.mailbox.name.try_into()?;

        let mut arg = None;
        let mut coroutine = ImapDelete::new(context, mailbox);

        loop {
            match coroutine.resume(arg.take()) {
                ImapDeleteResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                ImapDeleteResult::Ok { .. } => break,
                ImapDeleteResult::Err { err, .. } => bail!(err),
            }
        }

        printer.out(Message::new("Mailbox successfully deleted"))
    }
}
