//! # JMAP mailbox get
//!
//! The `jmap mailbox get` command, RFC 8621 `Mailbox/get`.

use anyhow::Result;
use clap::Parser;
use io_jmap::rfc8621::mailbox::get::JmapMailboxGetOptions;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    error::{ErrorCode, partial_when},
    jmap::{backend::get_errors, client::JmapClient, mailbox::query::JmapMailboxQueryOutput},
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
        let output = client.mailbox_get(JmapMailboxGetOptions {
            ids: Some(self.ids.clone()),
            properties: None,
        })?;

        let mailboxes = JmapMailboxQueryOutput {
            account: account.name().to_string(),
            page: None,
            mailboxes: output.mailboxes,
            next_page: None,
            errors: get_errors(
                output.not_found,
                output.unparsed,
                ErrorCode::MailboxNotFound,
                ErrorCode::MailboxUnreadable,
            ),
        };

        printer.out(&mailboxes)?;
        partial_when(&mailboxes.errors)
    }
}
