//! # Gmail language set
//!
//! The `gmail settings language set` command,
//! `users.settings.updateLanguage`.

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::{GmailLanguageSettings, update_language::GmailLanguageUpdate};
use pimalaya_cli::printer::{Message, Printer};

use crate::gmail::client::GmailClient;

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

        let _out = {
            let c = GmailLanguageUpdate::new(&client.auth, &client.user_id, settings)?;
            client.run(c)?
        };

        printer.out(Message::new("Gmail language settings successfully updated"))
    }
}
