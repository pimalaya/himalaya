//! # Gmail delegate list
//!
//! The `gmail settings delegates list` command,
//! `users.settings.delegates.list`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::delegates::list::{
    GmailDelegatesList, GmailDelegatesListResponse,
};
use pimalaya_cli::{columns::Columns, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    gmail::{client::GmailClient, settings::convert::verification_status_wire},
};

/// List all Gmail delegates (users.settings.delegates.list).
#[derive(Debug, Parser)]
pub struct GmailSettingsDelegatesListCommand;

impl GmailSettingsDelegatesListCommand {
    /// Lists every delegate and prints it.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        _account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        let out = {
            let c = GmailDelegatesList::new(&client.auth, &client.user_id)?;
            client.run(c)?
        };
        let resp = out.response;

        printer.out(GmailSettingsDelegateListOutput { response: resp })
    }
}

/// The `gmail settings delegates list` output, the Gmail delegates.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GmailSettingsDelegateListOutput {
    #[serde(flatten)]
    response: GmailDelegatesListResponse,
}

impl fmt::Display for GmailSettingsDelegateListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut columns = Columns::new(["EMAIL", "VERIFICATION"]);

        for delegate in &self.response.delegates {
            let status = delegate
                .verification_status
                .map(verification_status_wire)
                .unwrap_or_default();

            columns.push([&*sanitize::text(&delegate.delegate_email), status]);
        }

        write!(f, "{columns}")
    }
}
