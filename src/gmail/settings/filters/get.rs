//! # Gmail filter get
//!
//! The `gmail settings filters get` command,
//! `users.settings.filters.get`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::filters::{GmailFilter, get::GmailFilterGet};
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::gmail::{
    client::GmailClient,
    settings::filters::summary::{action_summary, criteria_summary},
};

/// Get a Gmail filter by identifier (users.settings.filters.get).
#[derive(Debug, Parser)]
pub struct GmailSettingsFilterGetCommand {
    /// Identifier of the filter to get.
    #[arg(value_name = "ID")]
    pub id: String,
}

impl GmailSettingsFilterGetCommand {
    /// Fetches the filter and tables it.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let out = {
            let c = GmailFilterGet::new(&client.auth, &client.user_id, &self.id)?;
            client.run(c)?
        };
        let filter = out.response;

        printer.out(GmailSettingsFilterGetOutput(filter))
    }
}

/// The `gmail settings filters get` output.
///
/// The text is a one-line summary of the criteria and the action, lossy on
/// purpose. The resource is emitted verbatim, so one filter has the very
/// same shape here as in a `list` row.
#[derive(Serialize, JsonSchema)]
#[serde(transparent)]
pub(crate) struct GmailSettingsFilterGetOutput(GmailFilter);

impl fmt::Display for GmailSettingsFilterGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Id: {}", self.0.id)?;

        if let Some(criteria) = &self.0.criteria {
            let summary = criteria_summary(criteria);
            if !summary.is_empty() {
                writeln!(f, "Criteria: {summary}")?;
            }
        }

        if let Some(action) = &self.0.action {
            let summary = action_summary(action);
            if !summary.is_empty() {
                writeln!(f, "Action: {summary}")?;
            }
        }

        Ok(())
    }
}
