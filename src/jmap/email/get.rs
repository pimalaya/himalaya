//! # JMAP email get
//!
//! The `jmap email get` command, RFC 8621 `Email/get`.

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    error::{ErrorCode, partial_when},
    jmap::{backend::get_errors, client::JmapClient, email::query::JmapEmailQueryOutput},
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
        let output = client.email_get(self.ids.clone(), Default::default())?;

        let mut emails = JmapEmailQueryOutput::new(account, output.emails);
        emails.errors = get_errors(
            output.not_found,
            output.unparsed,
            ErrorCode::MessageNotFound,
            ErrorCode::MessageUnreadable,
        );

        printer.out(&emails)?;
        partial_when(&emails.errors)
    }
}
