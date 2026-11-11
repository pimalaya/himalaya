//! # Gmail send-as list
//!
//! The `gmail settings sendas list` command,
//! `users.settings.sendAs.list`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::send_as::{GmailSendAs, list::GmailSendAsList};
use pimalaya_cli::{columns::Columns, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    gmail::{client::GmailClient, settings::convert::verification_status_wire},
};

/// List all Gmail send-as aliases (settings.sendAs.list).
#[derive(Debug, Parser)]
pub struct GmailSettingsSendAsListCommand;

impl GmailSettingsSendAsListCommand {
    /// Lists every send-as alias and prints it.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        _account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        let out = {
            let c = GmailSendAsList::new(&client.auth, &client.user_id)?;
            client.run(c)?
        };

        printer.out(GmailSettingsSendAsListOutput {
            send_as: out.response.send_as,
        })
    }
}

/// The `gmail settings sendas list` output, the Gmail send-as aliases.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GmailSettingsSendAsListOutput {
    send_as: Vec<GmailSendAs>,
}

impl fmt::Display for GmailSettingsSendAsListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // NOTE: a display name is free text, so it reads last.
        let mut columns = Columns::new(["EMAIL", "DEFAULT", "VERIFICATION", "NAME"]);

        for send_as in &self.send_as {
            let default = if send_as.is_default == Some(true) {
                "yes"
            } else {
                ""
            };
            let status = send_as
                .verification_status
                .map(verification_status_wire)
                .unwrap_or_default();
            let name = send_as.display_name.as_deref().unwrap_or_default();

            columns.push([
                &*sanitize::text(&send_as.send_as_email),
                default,
                status,
                &*sanitize::text(name),
            ]);
        }

        write!(f, "{columns}")
    }
}
