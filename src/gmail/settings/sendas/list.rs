//! # Gmail send-as list
//!
//! The `gmail settings sendas list` command,
//! `users.settings.sendAs.list`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::send_as::GmailSendAs;
use pimalaya_cli::{listing::Listing, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::{
    account::context::Account,
    gmail::{client::GmailClient, settings::convert::verification_status_wire},
    unified::backend::Backend,
};

/// List all Gmail send-as aliases (settings.sendAs.list).
///
/// JSON output: Gmail's answer as Gmail sent it, `{"sendAs": [<Gmail
/// send-as>...]}`.
#[derive(Debug, Parser)]
pub struct GmailSettingsSendAsListCommand;

impl GmailSettingsSendAsListCommand {
    /// Lists every send-as alias and prints it.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        let page = client.raw_list_at(&["settings", "sendAs"], "sendAs", Vec::new())?;

        printer.out(GmailSettingsSendAsListOutput {
            account: account.name().to_string(),
            send_as: page.items,
            rest: page.rest,
        })
    }
}

/// The `gmail settings sendas list` output, the Gmail send-as aliases.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GmailSettingsSendAsListOutput {
    #[serde(skip)]
    account: String,
    /// The send-as aliases, each as Gmail sent it.
    #[schemars(with = "Vec<Map<String, Value>>")]
    send_as: Vec<Value>,
    /// The rest of Gmail's answer as Gmail sent it.
    #[serde(flatten)]
    rest: Map<String, Value>,
}

impl fmt::Display for GmailSettingsSendAsListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // NOTE: a display name is free text, so it reads last.
        let mut listing = Listing::new(&self.account, Some(Backend::Gmail.as_str()));

        for send_as in &self.send_as {
            let send_as: GmailSendAs = serde_json::from_value(send_as.clone()).unwrap_or_default();
            let default = if send_as.is_default == Some(true) {
                "default"
            } else {
                ""
            };
            let status = send_as
                .verification_status
                .map(verification_status_wire)
                .unwrap_or_default();
            let name = send_as.display_name.as_deref().unwrap_or_default();

            listing.items.push([
                default.to_string(),
                status.to_string(),
                sanitize::text(&send_as.send_as_email).into_owned(),
                sanitize::text(name).into_owned(),
            ]);
        }

        write!(f, "{listing}")
    }
}
