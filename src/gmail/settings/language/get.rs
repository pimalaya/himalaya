//! # Gmail language get
//!
//! The `gmail settings language get` command,
//! `users.settings.getLanguage`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::{GmailLanguageSettings, get_language::GmailLanguageGet};
use pimalaya_cli::{fields::Fields, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::gmail::client::GmailClient;

/// Get the Gmail display language settings.
#[derive(Debug, Parser)]
pub struct GmailSettingsLanguageGetCommand;

impl GmailSettingsLanguageGetCommand {
    /// Fetches the display language and prints it.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let out = {
            let c = GmailLanguageGet::new(&client.auth, &client.user_id)?;
            client.run(c)?
        };
        printer.out(GmailSettingsLanguageGetOutput::from(out.response))
    }
}

impl From<GmailLanguageSettings> for GmailSettingsLanguageGetOutput {
    fn from(settings: GmailLanguageSettings) -> Self {
        Self {
            display_language: settings.display_language,
        }
    }
}

/// Gmail display language settings, rendered as text or, under `--json`,
/// as a structured object instead of a wrapped human string.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GmailSettingsLanguageGetOutput {
    display_language: String,
}

impl fmt::Display for GmailSettingsLanguageGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut fields = Fields::default();
        fields.push("Display language", sanitize::text(&self.display_language));
        write!(f, "{fields}")
    }
}
