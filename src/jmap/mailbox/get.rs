//! # JMAP mailbox get
//!
//! The `jmap mailbox get` command, RFC 8621 `Mailbox/get`.

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    error::{ErrorCode, partial_when},
    jmap::{
        backend::not_found_errors, client::JmapClient, mailbox::query::JmapMailboxQueryOutput, raw,
    },
};

/// Get JMAP mailboxes by ID (Mailbox/get).
///
/// An id the server does not find is an entry of `errors`, coded
/// `mailbox-not-found`, and exits 3.
#[derive(Debug, Parser)]
pub struct JmapMailboxGetCommand {
    /// Mailbox ID(s) to retrieve.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,
}

impl JmapMailboxGetCommand {
    /// Fetches the mailboxes and lists them, then the error of each id not
    /// found.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut JmapClient,
    ) -> Result<()> {
        let output = raw::get(client, "Mailbox", Some(&self.ids))?;

        let mailboxes = JmapMailboxQueryOutput {
            account: account.name().to_string(),
            page: None,
            mailboxes: output.list,
            rest: output.rest,
            next_page: None,
            errors: not_found_errors(output.not_found, ErrorCode::MailboxNotFound),
        };

        printer.out(&mailboxes)?;
        partial_when(&mailboxes.errors)
    }
}
