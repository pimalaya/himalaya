//! # JMAP email get
//!
//! The `jmap email get` command, RFC 8621 `Email/get`.

use anyhow::Result;
use clap::Parser;
use log::warn;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    jmap::{client::JmapClient, email::query::JmapEmailQueryOutput},
};

/// Get JMAP emails by ID (Email/get).
///
/// Fetches and lists their envelopes.
#[derive(Debug, Parser)]
pub struct JmapEmailGetCommand {
    /// The email ID(s) to retrieve.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,
}

impl JmapEmailGetCommand {
    /// Fetches the emails and lists their envelopes.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut JmapClient,
    ) -> Result<()> {
        let output = client.email_get(self.ids.clone(), Default::default())?;

        for id in output.not_found {
            warn!("email `{id}` not found, ignoring it");
        }

        printer.out(JmapEmailQueryOutput::new(account, output.emails))
    }
}
