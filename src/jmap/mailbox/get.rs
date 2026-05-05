use anyhow::Result;
use clap::Parser;
use log::warn;
use pimalaya_cli::printer::Printer;

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
        let mut client = account.new_jmap_client()?;
        let output = client.mailbox_get(Some(self.ids.clone()), None)?;

        for id in output.not_found {
            warn!("mailbox `{id}` not found, ignoring it");
        }

        let table = MailboxesTable {
            preset: account.table_preset,
            mailboxes: output.mailboxes,
        };

        printer.out(table)
    }
}
