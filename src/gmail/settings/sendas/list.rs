//! # Gmail send-as list
//!
//! The `gmail settings sendas list` command,
//! `users.settings.sendAs.list`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::send_as::{GmailSendAs, list::GmailSendAsList};
use pimalaya_cli::{printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    backend::Backend,
    gmail::{client::GmailClient, settings::convert::verification_status_wire},
    listing::Listing,
};

/// List all Gmail send-as aliases (settings.sendAs.list).
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
        let out = {
            let c = GmailSendAsList::new(&client.auth, &client.user_id)?;
            client.run(c)?
        };

        printer.out(GmailSettingsSendAsListOutput {
            account: account.name().to_string(),
            send_as: out.response.send_as,
        })
    }
}

/// The `gmail settings sendas list` output, the Gmail send-as aliases.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GmailSettingsSendAsListOutput {
    #[serde(skip)]
    account: String,
    send_as: Vec<GmailSendAs>,
}

impl fmt::Display for GmailSettingsSendAsListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // NOTE: a display name is free text, so it reads last.
        let mut listing = Listing::new(&self.account, Some(Backend::Gmail));

        for send_as in &self.send_as {
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
                default,
                status,
                &*sanitize::text(&send_as.send_as_email),
                &*sanitize::text(name),
            ]);
        }

        write!(f, "{listing}")
    }
}
