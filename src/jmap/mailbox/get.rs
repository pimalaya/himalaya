use anyhow::Result;
use clap::Parser;
use io_jmap::rfc8621::mailbox::get::JmapMailboxGetOptions;
use log::warn;
use pimalaya_cli::printer::Printer;

use crate::account::context::Account;
use crate::jmap::{
    client::JmapClient,
    mailbox::query::{MailboxColors, MailboxesTable},
};

/// Get JMAP mailboxes by ID (Mailbox/get).
#[derive(Debug, Parser)]
pub struct JmapMailboxGetCommand {
    /// Mailbox ID(s) to retrieve.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,
}

impl JmapMailboxGetCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut JmapClient,
    ) -> Result<()> {
        let output = client.mailbox_get(JmapMailboxGetOptions {
            ids: Some(self.ids.clone()),
            properties: None,
        })?;

        for id in output.not_found {
            warn!("mailbox `{id}` not found, ignoring it");
        }

        let table = MailboxesTable {
            preset: account.table_preset().to_string(),
            colors: MailboxColors {
                id: account.mailboxes_list_table_id_color(),
                name: account.mailboxes_list_table_name_color(),
                total: account.mailboxes_list_table_total_color(),
                unread: account.mailboxes_list_table_unread_color(),
            },
            mailboxes: output.mailboxes,
        };

        printer.out(table)
    }
}
