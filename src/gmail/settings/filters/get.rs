//! # Gmail filter get
//!
//! The `gmail settings filters get` command,
//! `users.settings.filters.get`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::filters::GmailFilter;
use pimalaya_cli::{fields::Fields, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Map, Value};

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
    /// Fetches the filter and prints it.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let filter = client.raw_get(&["settings", "filters", &self.id])?;
        printer.out(GmailSettingsFilterGetOutput(Value::Object(filter)))
    }
}

/// The `gmail settings filters get` output.
///
/// The text is a one-line summary of the criteria and the action, lossy on
/// purpose. The resource is emitted verbatim, so one filter has the very
/// same shape here as in a `list` row.
#[derive(Serialize, JsonSchema)]
#[serde(transparent)]
pub(crate) struct GmailSettingsFilterGetOutput(#[schemars(with = "Map<String, Value>")] Value);

impl fmt::Display for GmailSettingsFilterGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let filter: GmailFilter = serde_json::from_value(self.0.clone()).unwrap_or_default();
        let mut fields = Fields::default();
        fields.push("Id", sanitize::text(&filter.id));

        if let Some(criteria) = &filter.criteria {
            let summary = criteria_summary(criteria);
            if !summary.is_empty() {
                fields.push("Criteria", summary);
            }
        }

        if let Some(action) = &filter.action {
            let summary = action_summary(action);
            if !summary.is_empty() {
                fields.push("Action", summary);
            }
        }

        write!(f, "{fields}")
    }
}
