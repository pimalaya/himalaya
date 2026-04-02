use anyhow::{bail, Result};
use clap::Parser;
use io_imap::rfc3501::subscribe::*;
use io_socket::runtimes::std_stream::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::imap::{account::ImapAccount, mailbox::arg::MailboxNameArg};

/// Subscribe to the given mailbox.
///
/// This command subscribes to a mailbox, making it appear in the
/// list of subscribed mailboxes.
#[derive(Debug, Parser)]
pub struct ImapMailboxSubscribeCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameArg,
}

impl ImapMailboxSubscribeCommand {
    pub fn execute(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let mut imap = account.new_imap_session()?;
        let mailbox = self.mailbox_name.inner.try_into()?;

        let mut arg = None;
        let mut coroutine = ImapMailboxSubscribe::new(imap.context, mailbox);

        loop {
            match coroutine.resume(arg.take()) {
                ImapMailboxSubscribeResult::Io { input } => {
                    arg = Some(handle(&mut imap.stream, input)?)
                }
                ImapMailboxSubscribeResult::Ok { .. } => break,
                ImapMailboxSubscribeResult::Err { err, .. } => bail!(err),
            }
        }

        printer.out(Message::new("Mailbox successfully subscribed"))
    }
}
