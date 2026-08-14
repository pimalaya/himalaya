use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::get_language::GmailLanguageGet;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::gmail::client::GmailClient;

/// Get the Gmail display language settings.
#[derive(Debug, Parser)]
pub struct GmailSettingsLanguageGetCommand;

impl GmailSettingsLanguageGetCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let out = {
            let c = GmailLanguageGet::new(&client.auth, &client.user_id)?;
            client.run(c)?
        };
        let settings = out.response;

        printer.out(GmailSettingsLanguageGetOutput {
            display_language: settings.display_language,
        })
    }
}

/// Gmail display language settings, rendered as text or, under `--json`,
/// as a structured object instead of a wrapped human string.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct GmailSettingsLanguageGetOutput {
    display_language: String,
}

impl fmt::Display for GmailSettingsLanguageGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Display language: {}", self.display_language)
    }
}
