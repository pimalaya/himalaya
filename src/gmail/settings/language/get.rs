//! # Gmail language get
//!
//! The `gmail settings language get` command,
//! `users.settings.getLanguage`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::GmailLanguageSettings;
use pimalaya_cli::{fields::Fields, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::gmail::client::GmailClient;

/// Get the Gmail display language settings.
#[derive(Debug, Parser)]
pub struct GmailSettingsLanguageGetCommand;

impl GmailSettingsLanguageGetCommand {
    /// Fetches the display language and prints it.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let settings = client.raw_get(&["settings", "language"])?;
        printer.out(GmailSettingsLanguageGetOutput(Value::Object(settings)))
    }
}

impl From<GmailLanguageSettings> for View {
    fn from(settings: GmailLanguageSettings) -> Self {
        Self {
            display_language: settings.display_language,
        }
    }
}

/// The fields the text form prints, read from what Gmail sent.
struct View {
    display_language: String,
}

impl fmt::Display for View {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut fields = Fields::default();
        fields.push("Display language", sanitize::text(&self.display_language));
        write!(f, "{fields}")
    }
}

/// The settings as Gmail sent them, every property kept, the text form
/// reading what it shows out of them.
#[derive(Serialize, JsonSchema)]
#[serde(transparent)]
pub(crate) struct GmailSettingsLanguageGetOutput(
    #[schemars(with = "Map<String, Value>")] pub(crate) Value,
);

impl fmt::Display for GmailSettingsLanguageGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let settings: GmailLanguageSettings =
            serde_json::from_value(self.0.clone()).unwrap_or_default();
        View::from(settings).fmt(f)
    }
}
