//! # JMAP mailbox get
//!
//! The `jmap mailbox get` command, RFC 8621 `Mailbox/get`.

use anyhow::Result;
use clap::Parser;
use io_jmap::rfc8621::mailbox::get::JmapMailboxGetOptions;
use log::warn;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    jmap::{client::JmapClient, mailbox::query::JmapMailboxQueryOutput},
};

/// Get JMAP mailboxes by ID (Mailbox/get).
#[derive(Debug, Parser)]
pub struct JmapMailboxGetCommand {
    /// Mailbox ID(s) to retrieve.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,
}

impl JmapMailboxGetCommand {
    /// Fetches the mailboxes and lists them.
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

        printer.out(JmapMailboxQueryOutput {
            account: account.name().to_string(),
            page: None,
            mailboxes: output.mailboxes,
        })
    }
}
