use anyhow::{bail, Result};
use clap::Parser;
use io_jmap::coroutines::mailbox_get::{GetJmapMailboxes, GetJmapMailboxesResult};
use io_stream::runtimes::std::handle;
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

        let mut coroutine = GetJmapMailboxes::new(jmap.context, Some(self.ids.clone()), None)?;
        let mut arg = None;

        let (mailboxes, not_found) = loop {
            match coroutine.resume(arg.take()) {
                GetJmapMailboxesResult::Io(io) => arg = Some(handle(&mut jmap.stream, io)?),
                GetJmapMailboxesResult::Ok {
                    mailboxes,
                    not_found,
                    ..
                } => break (mailboxes, not_found),
                GetJmapMailboxesResult::Err { err, .. } => bail!(err),
            }
        };

        for id in not_found {
            warn!("mailbox `{id}` not found");
        }

        let table = MailboxesTable {
            preset: account.table_preset,
            mailboxes,
        };

        printer.out(table)
    }
}
