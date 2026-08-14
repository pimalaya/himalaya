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
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let out = {
            let c = GmailFilterGet::new(&client.auth, &client.user_id, &self.id)?;
            client.run(c)?
        };
        let filter = out.response;

        printer.out(GmailSettingsFilterGetOutput(filter))
    }
}

/// A Gmail filter, rendered as a one-line summary of its criteria and
/// action or, under `--json`, as the filter resource itself instead of a
/// wrapped human string.
///
/// The resource is emitted verbatim so that one filter read with `get`
/// has the very same shape as a row of `list`, and so that the criteria
/// and action stay machine-readable: the summaries the text rendering
/// builds are lossy on purpose.
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
