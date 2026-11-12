//! # Gmail filter list
//!
//! The `gmail settings filters list` command,
//! `users.settings.filters.list`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::filters::list::{GmailFiltersList, GmailFiltersListResponse};
use pimalaya_cli::{printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    backend::Backend,
    gmail::{
        client::GmailClient,
        settings::filters::summary::{action_summary, criteria_summary},
    },
    listing::Listing,
};

/// List all Gmail filters (users.settings.filters.list).
#[derive(Debug, Parser)]
pub struct GmailSettingsFiltersListCommand;

impl GmailSettingsFiltersListCommand {
    /// Lists every filter and prints it.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        let out = {
            let c = GmailFiltersList::new(&client.auth, &client.user_id)?;
            client.run(c)?
        };
        let resp = out.response;

        printer.out(GmailSettingsFilterListOutput {
            account: account.name().to_string(),
            response: resp,
        })
    }
}

/// The `gmail settings filters list` output, the Gmail filters.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GmailSettingsFilterListOutput {
    #[serde(skip)]
    account: String,
    #[serde(flatten)]
    response: GmailFiltersListResponse,
}

impl fmt::Display for GmailSettingsFilterListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.account, Some(Backend::Gmail));

        for filter in &self.response.filter {
            let criteria = filter.criteria.as_ref().map(criteria_summary);
            let action = filter.action.as_ref().map(action_summary);

            listing.items.push([
                sanitize::text(&filter.id).into_owned(),
                criteria.unwrap_or_default(),
                action.unwrap_or_default(),
            ]);
        }

        write!(f, "{listing}")
    }
}
