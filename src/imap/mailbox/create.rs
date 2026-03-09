use anyhow::{bail, Result};
use clap::Parser;
use io_imap::coroutines::create::*;
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::imap::{account::ImapAccount, mailbox::arg::MailboxNameArg, stream};

/// Create the given mailbox.
///
/// This command allows you to create a new mailbox using the given
/// name.
#[derive(Debug, Parser)]
pub struct CreateMailboxCommand {
    #[command(flatten)]
    pub mailbox: MailboxNameArg,
}

impl CreateMailboxCommand {
    pub fn exec(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let (context, mut stream) = stream::connect(account.backend)?;

        let mailbox = self.mailbox.name.try_into()?;

        let mut arg = None;
        let mut coroutine = ImapCreate::new(context, mailbox);

        loop {
            match coroutine.resume(arg.take()) {
                ImapCreateResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                ImapCreateResult::Ok { .. } => break,
                ImapCreateResult::Err { err, .. } => bail!(err),
            }
        }

        printer.out(Message::new("Mailbox successfully created"))
    }
}
