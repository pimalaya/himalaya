//! # JMAP email get
//!
//! The `jmap email get` command, RFC 8621 `Email/get`.

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    error::{ErrorCode, partial_when},
    jmap::{
        backend::not_found_errors, client::JmapClient, email::query::JmapEmailQueryOutput, raw,
    },
};

/// Get JMAP emails by ID (Email/get).
///
/// Fetches and lists their envelopes. An id the server does not find is
/// an entry of `errors`, coded `message-not-found`, and exits 3.
#[derive(Debug, Parser)]
pub struct JmapEmailGetCommand {
    /// The email ID(s) to retrieve.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,
}

impl JmapEmailGetCommand {
    /// Fetches the emails and lists their envelopes, then the error of
    /// each id not found.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut JmapClient,
    ) -> Result<()> {
        let output = raw::get(client, "Email", Some(&self.ids))?;

        let mut emails = JmapEmailQueryOutput::new(account, output.list, output.rest);
        emails.errors = not_found_errors(output.not_found, ErrorCode::MessageNotFound);

        printer.out(&emails)?;
        partial_when(&emails.errors)
    }
}
