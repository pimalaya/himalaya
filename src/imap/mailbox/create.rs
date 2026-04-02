use anyhow::{bail, Result};
use clap::Parser;
use io_imap::rfc3501::create::*;
use io_socket::runtimes::std_stream::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::imap::{account::ImapAccount, mailbox::arg::MailboxNameArg};

/// Create the given mailbox.
///
/// This command allows you to create a new mailbox using the given
/// name.
#[derive(Debug, Parser)]
pub struct ImapMailboxCreateCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameArg,
}

impl ImapMailboxCreateCommand {
    pub fn execute(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let mut imap = account.new_imap_session()?;

        let mailbox = self.mailbox_name.inner.try_into()?;

        let mut arg = None;
        let mut coroutine = ImapMailboxCreate::new(imap.context, mailbox);

        loop {
            match coroutine.resume(arg.take()) {
                ImapMailboxCreateResult::Io { input } => {
                    arg = Some(handle(&mut imap.stream, input)?)
                }
                ImapMailboxCreateResult::Ok { .. } => break,
                ImapMailboxCreateResult::Err { err, .. } => bail!(err),
            }
        }

        printer.out(Message::new("Mailbox successfully created"))
    }
}
