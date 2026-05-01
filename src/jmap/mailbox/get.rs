use std::io::{Read, Write};

use anyhow::{bail, Result};
use clap::Parser;
use io_jmap::rfc8621::mailbox_get::{JmapMailboxGet, JmapMailboxGetResult};
use log::warn;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::jmap::{account::JmapAccount, mailbox::query::MailboxesTable};

const READ_BUFFER_SIZE: usize = 16 * 1024;

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
        let mut buf = [0u8; READ_BUFFER_SIZE];
        let mut arg: Option<&[u8]> = None;

        let (mailboxes, not_found) = loop {
            match coroutine.resume(arg.take()) {
                JmapMailboxGetResult::Ok {
                    mailboxes,
                    not_found,
                    ..
                } => break (mailboxes, not_found),
                JmapMailboxGetResult::WantsRead => {
                    let n = jmap.stream.read(&mut buf)?;
                    arg = Some(&buf[..n]);
                }
                JmapMailboxGetResult::WantsWrite(bytes) => {
                    jmap.stream.write_all(&bytes)?;
                    arg = None;
                }
                JmapMailboxGetResult::Err(err) => bail!("{err}"),
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
