use anyhow::{bail, Result};
use clap::Parser;
use io_jmap::rfc8621::coroutines::mailbox_get::{JmapMailboxGet, JmapMailboxGetResult};
use io_socket::runtimes::std_stream::handle;
use log::warn;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::jmap::{account::JmapAccount, mailbox::query::MailboxesTable};

/// Get JMAP mailboxes by ID (Mailbox/get).
#[derive(Debug, Parser)]
pub struct JmapMailboxGetCommand {
    /// Mailbox ID(s) to retrieve.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,
}

impl JmapMailboxGetCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;

        let mut coroutine =
            JmapMailboxGet::new(&jmap.session, &jmap.http_auth, Some(self.ids.clone()), None)?;
        let mut arg = None;

        let (mailboxes, not_found) = loop {
            match coroutine.resume(arg.take()) {
                JmapMailboxGetResult::Io { io } => arg = Some(handle(&mut jmap.stream, io)?),
                JmapMailboxGetResult::Ok {
                    mailboxes,
                    not_found,
                    ..
                } => break (mailboxes, not_found),
                JmapMailboxGetResult::Err { err, .. } => bail!(err),
            }
        };

        for id in not_found {
            warn!("mailbox `{id}` not found, ignoring it");
        }

        let table = MailboxesTable {
            preset: account.table_preset,
            mailboxes,
        };

        printer.out(table)
    }
}
