//! # Gmail delegate get
//!
//! The `gmail settings delegates get` command,
//! `users.settings.delegates.get`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::delegates::{GmailDelegate, get::GmailDelegateGet};
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::gmail::{client::GmailClient, settings::convert::verification_status_wire};

/// Get a Gmail delegate by email address (users.settings.delegates.get).
#[derive(Debug, Parser)]
pub struct GmailSettingsDelegateGetCommand {
    /// Email address of the delegate to get.
    #[arg(value_name = "EMAIL")]
    pub email: String,
}

impl GmailSettingsDelegateGetCommand {
    /// Fetches the delegate and tables it.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let out = {
            let c = GmailDelegateGet::new(&client.auth, &client.user_id, &self.email)?;
            client.run(c)?
        };
        let delegate = out.response;

        printer.out(GmailSettingsDelegateGetOutput(delegate))
    }
}

/// The `gmail settings delegates get` output.
///
/// The resource is emitted verbatim, so one delegate has the very same
/// shape here as in a `list` row.
#[derive(Serialize, JsonSchema)]
#[serde(transparent)]
pub(crate) struct GmailSettingsDelegateGetOutput(GmailDelegate);

impl fmt::Display for GmailSettingsDelegateGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Email: {}", self.0.delegate_email)?;

        if let Some(status) = self.0.verification_status {
            writeln!(f, "Verification: {}", verification_status_wire(status))?;
        }

        Ok(())
    }
}
