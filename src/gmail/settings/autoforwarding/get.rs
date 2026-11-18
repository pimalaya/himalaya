//! # Gmail auto-forwarding get
//!
//! The `gmail settings autoforwarding get` command,
//! `users.settings.getAutoForwarding`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::GmailAutoForwarding;
use pimalaya_cli::{fields::Fields, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::gmail::{
    client::GmailClient,
    settings::convert::{disposition_wire, yes_no},
};

/// Get the Gmail auto-forwarding settings.
#[derive(Debug, Parser)]
pub struct GmailSettingsAutoForwardingGetCommand;

impl GmailSettingsAutoForwardingGetCommand {
    /// Fetches the auto-forwarding settings and prints them.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let settings = client.raw_get(&["settings", "autoForwarding"])?;
        printer.out(GmailSettingsAutoForwardingGetOutput(Value::Object(
            settings,
        )))
    }
}

impl From<GmailAutoForwarding> for View {
    fn from(settings: GmailAutoForwarding) -> Self {
        Self {
            enabled: settings.enabled,
            email_address: settings.email_address,
            disposition: settings
                .disposition
                .map(|disposition| disposition_wire(disposition).to_string()),
        }
    }
}

/// The fields the text form prints, read from what Gmail sent.
struct View {
    enabled: bool,
    email_address: Option<String>,
    disposition: Option<String>,
}

impl fmt::Display for View {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut fields = Fields::default();
        fields.push("Enabled", yes_no(self.enabled));

        if let Some(email_address) = &self.email_address {
            fields.push("Email address", sanitize::text(email_address));
        }
        if let Some(disposition) = &self.disposition {
            fields.push("Disposition", disposition);
        }

        write!(f, "{fields}")
    }
}

/// The settings as Gmail sent them, every property kept, the text form
/// reading what it shows out of them.
#[derive(Serialize, JsonSchema)]
#[serde(transparent)]
pub(crate) struct GmailSettingsAutoForwardingGetOutput(
    #[schemars(with = "Map<String, Value>")] pub(crate) Value,
);

impl fmt::Display for GmailSettingsAutoForwardingGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let settings: GmailAutoForwarding =
            serde_json::from_value(self.0.clone()).unwrap_or_default();
        View::from(settings).fmt(f)
    }
}
