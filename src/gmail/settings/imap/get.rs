//! # Gmail IMAP get
//!
//! The `gmail settings imap get` command, `users.settings.getImap`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::GmailImapSettings;
use pimalaya_cli::{fields::Fields, printer::Printer};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::gmail::{
    client::GmailClient,
    settings::convert::{expunge_behavior_wire, yes_no},
};

/// Get the Gmail IMAP access settings.
#[derive(Debug, Parser)]
pub struct GmailSettingsImapGetCommand;

impl GmailSettingsImapGetCommand {
    /// Fetches the IMAP settings and prints them.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let settings = client.raw_get(&["settings", "imap"])?;
        printer.out(GmailSettingsImapGetOutput(Value::Object(settings)))
    }
}

impl From<GmailImapSettings> for View {
    fn from(settings: GmailImapSettings) -> Self {
        Self {
            enabled: settings.enabled,
            auto_expunge: settings.auto_expunge,
            expunge_behavior: settings
                .expunge_behavior
                .map(|behavior| expunge_behavior_wire(behavior).to_string()),
            max_folder_size: settings.max_folder_size,
        }
    }
}

/// The fields the text form prints, read from what Gmail sent.
struct View {
    enabled: bool,
    auto_expunge: Option<bool>,
    expunge_behavior: Option<String>,
    max_folder_size: Option<u32>,
}

impl fmt::Display for View {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut fields = Fields::default();
        fields.push("Enabled", yes_no(self.enabled));

        if let Some(auto_expunge) = self.auto_expunge {
            fields.push("Auto expunge", yes_no(auto_expunge));
        }
        if let Some(behavior) = &self.expunge_behavior {
            fields.push("Expunge behavior", behavior);
        }
        if let Some(size) = self.max_folder_size {
            fields.push("Max folder size", size);
        }

        write!(f, "{fields}")
    }
}

/// The settings as Gmail sent them, every property kept, the text form
/// reading what it shows out of them.
#[derive(Serialize, JsonSchema)]
#[serde(transparent)]
pub(crate) struct GmailSettingsImapGetOutput(
    #[schemars(with = "Map<String, Value>")] pub(crate) Value,
);

impl fmt::Display for GmailSettingsImapGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let settings: GmailImapSettings =
            serde_json::from_value(self.0.clone()).unwrap_or_default();
        View::from(settings).fmt(f)
    }
}
