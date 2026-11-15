//! # Gmail delegate list
//!
//! The `gmail settings delegates list` command,
//! `users.settings.delegates.list`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::delegates::GmailDelegate;
use pimalaya_cli::{listing::Listing, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::{
    account::context::Account,
    backend::Backend,
    gmail::{client::GmailClient, settings::convert::verification_status_wire},
};

/// List all Gmail delegates (users.settings.delegates.list).
///
/// JSON output: Gmail's answer as Gmail sent it, `{"delegates": [<Gmail
/// delegate>...]}`, `[]` when there is none.
#[derive(Debug, Parser)]
pub struct GmailSettingsDelegatesListCommand;

impl GmailSettingsDelegatesListCommand {
    /// Lists every delegate and prints it.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        let page = client.raw_list_at(&["settings", "delegates"], "delegates", Vec::new())?;

        printer.out(GmailSettingsDelegateListOutput {
            account: account.name().to_string(),
            delegates: page.items,
            rest: page.rest,
        })
    }
}

/// The `gmail settings delegates list` output, the Gmail delegates.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GmailSettingsDelegateListOutput {
    #[serde(skip)]
    account: String,
    /// The delegates, each as Gmail sent it.
    #[schemars(with = "Vec<Map<String, Value>>")]
    delegates: Vec<Value>,
    /// The rest of Gmail's answer as Gmail sent it.
    #[serde(flatten)]
    rest: Map<String, Value>,
}

impl fmt::Display for GmailSettingsDelegateListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.account, Some(Backend::Gmail.as_str()));

        for delegate in &self.delegates {
            let delegate: GmailDelegate =
                serde_json::from_value(delegate.clone()).unwrap_or_default();
            let status = delegate
                .verification_status
                .map(verification_status_wire)
                .unwrap_or_default();

            listing.items.push([
                status.to_string(),
                sanitize::text(&delegate.delegate_email).into_owned(),
            ]);
        }

        write!(f, "{listing}")
    }
}
