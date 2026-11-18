//! # Gmail language set
//!
//! The `gmail settings language set` command,
//! `users.settings.updateLanguage`.

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::GmailLanguageSettings;
use pimalaya_cli::printer::Printer;

use crate::gmail::{
    client::GmailClient, settings::language::get::GmailSettingsLanguageGetOutput, settings::raw_put,
};

/// Update the Gmail display language settings.
#[derive(Debug, Parser)]
pub struct GmailSettingsLanguageSetCommand {
    /// Display language tag to set, such as `en` or `fr`.
    #[arg(long, value_name = "LANG")]
    pub display_language: String,
}

impl GmailSettingsLanguageSetCommand {
    /// Applies the display language.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let settings = GmailLanguageSettings {
            display_language: self.display_language,
        };

        let settings = raw_put(client, "language", &settings)?;
        printer.out(GmailSettingsLanguageGetOutput(settings))
    }
}
