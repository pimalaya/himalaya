//! # Gmail auto-forwarding get
//!
//! The `gmail settings autoforwarding get` command,
//! `users.settings.getAutoForwarding`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::get_auto_forwarding::GmailAutoForwardingGet;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::gmail::{
    client::GmailClient,
    settings::convert::{disposition_wire, yes_no},
};

/// Get the Gmail auto-forwarding settings.
#[derive(Debug, Parser)]
pub struct GmailSettingsAutoForwardingGetCommand;

impl GmailSettingsAutoForwardingGetCommand {
    /// Fetches the auto-forwarding settings and tables them.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let out = {
            let c = GmailAutoForwardingGet::new(&client.auth, &client.user_id)?;
            client.run(c)?
        };
        let settings = out.response;

        printer.out(GmailSettingsAutoForwardingGetOutput {
            enabled: settings.enabled,
            email_address: settings.email_address,
            disposition: settings
                .disposition
                .map(|disposition| disposition_wire(disposition).to_string()),
        })
    }
}

/// The `gmail settings autoforwarding get` output.
///
/// The disposition keeps its Gmail wire spelling, so a value `get` reads
/// is a value `set` accepts back.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct GmailSettingsAutoForwardingGetOutput {
    enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    email_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disposition: Option<String>,
}

impl fmt::Display for GmailSettingsAutoForwardingGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Enabled: {}", yes_no(self.enabled))?;

        if let Some(email_address) = &self.email_address {
            writeln!(f, "Email address: {email_address}")?;
        }
        if let Some(disposition) = &self.disposition {
            writeln!(f, "Disposition: {disposition}")?;
        }

        Ok(())
    }
}
