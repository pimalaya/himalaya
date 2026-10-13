//! # pimdir attachment search
//!
//! The `pimdir attachment search` command, the `pimdir attachment list`
//! page narrowed to the files whose name or sender match words.

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    pimdir::{
        attachment::{AttachmentQuery, list::PageArgs},
        client::PimdirClient,
    },
};

/// Search the files the account's messages attach by name and sender.
///
/// Every word must appear, ignoring case, in the file name, the sender's
/// address or the sender's name: `invoice alice` finds the invoices
/// Alice sent. Accents count, `é` matching `É` but not `e`.
///
/// The output and the paging are those of `pimdir attachment list`. The
/// store is read newest message first until the page is full, so a word
/// few files match reads further.
#[derive(Debug, Parser)]
pub struct PimdirAttachmentSearchCommand {
    /// The words to find.
    #[arg(value_name = "WORDS", required = true, num_args = 1..)]
    pub query: Vec<String>,
    #[command(flatten)]
    pub page: PageArgs,
}

impl PimdirAttachmentSearchCommand {
    /// Searches one page of the account's attachments and prints it.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut PimdirClient,
    ) -> Result<()> {
        let query = AttachmentQuery::new(&self.query.join(" "))?;
        self.page.print(printer, account, client, Some(&query))
    }
}
