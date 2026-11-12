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
use pimalaya_cli::{printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    backend::Backend,
    gmail::{client::GmailClient, settings::convert::verification_status_wire},
    listing::Listing,
};

/// List all Gmail delegates (users.settings.delegates.list).
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
        let out = {
            let c = GmailDelegatesList::new(&client.auth, &client.user_id)?;
            client.run(c)?
        };
        let resp = out.response;

        printer.out(GmailSettingsDelegateListOutput {
            account: account.name().to_string(),
            response: resp,
        })
    }
}

/// The `gmail settings delegates list` output, the Gmail delegates.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GmailSettingsDelegateListOutput {
    #[serde(skip)]
    account: String,
    #[serde(flatten)]
    response: GmailDelegatesListResponse,
}

impl fmt::Display for GmailSettingsDelegateListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.account, Some(Backend::Gmail));

        for delegate in &self.response.delegates {
            let status = delegate
                .verification_status
                .map(verification_status_wire)
                .unwrap_or_default();

            listing
                .items
                .push([status, &*sanitize::text(&delegate.delegate_email)]);
        }

        write!(f, "{listing}")
    }
}
