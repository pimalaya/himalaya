use anyhow::{bail, Result};
use clap::Parser;
use io_imap::rfc3501::delete::*;
use io_socket::runtimes::std_stream::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::imap::{account::ImapAccount, mailbox::arg::MailboxNameArg};

/// Delete the given mailbox.
///
/// All emails from the given mailbox are definitely deleted. The
/// mailbox is also deleted after execution of the command.
#[derive(Debug, Parser)]
pub struct ImapMailboxDeleteCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameArg,
}

impl ImapMailboxDeleteCommand {
    pub fn execute(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let mut imap = account.new_imap_session()?;
        let mailbox = self.mailbox_name.inner.try_into()?;

        let mut arg = None;
        let mut coroutine = ImapMailboxDelete::new(imap.context, mailbox);

        loop {
            match coroutine.resume(arg.take()) {
                ImapMailboxDeleteResult::Io { input } => {
                    arg = Some(handle(&mut imap.stream, input)?)
                }
                ImapMailboxDeleteResult::Ok { .. } => break,
                ImapMailboxDeleteResult::Err { err, .. } => bail!(err),
            }
        }

        printer.out(Message::new("Mailbox successfully deleted"))
    }
}
