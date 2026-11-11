//! # Gmail filter list
//!
//! The `gmail settings filters list` command,
//! `users.settings.filters.list`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::filters::list::{GmailFiltersList, GmailFiltersListResponse};
use pimalaya_cli::{
    fields::Fields,
    printer::Printer,
    records::{Record, Records},
    sanitize,
};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    gmail::{
        client::GmailClient,
        settings::filters::summary::{action_summary, criteria_summary},
    },
};

/// List all Gmail filters (users.settings.filters.list).
#[derive(Debug, Parser)]
pub struct GmailSettingsFiltersListCommand;

impl GmailSettingsFiltersListCommand {
    /// Lists every filter and prints it.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        _account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        let out = {
            let c = GmailFiltersList::new(&client.auth, &client.user_id)?;
            client.run(c)?
        };
        let resp = out.response;

        printer.out(GmailSettingsFilterListOutput { response: resp })
    }
}

/// The `gmail settings filters list` output, the Gmail filters.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GmailSettingsFilterListOutput {
    #[serde(flatten)]
    response: GmailFiltersListResponse,
}

impl fmt::Display for GmailSettingsFilterListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut records = Records::default();

        for filter in &self.response.filter {
            let mut fields = Fields::default();

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

            records.push(Record {
                title: sanitize::text(&filter.id).into_owned(),
                state: Vec::new(),
                fields,
                body: String::new(),
            });
        }

        write!(f, "{records}")
    }
}
