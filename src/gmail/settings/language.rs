use anyhow::Result;
use clap::{Parser, Subcommand};
use io_gmail::v1::rest::settings::{
    GmailLanguageSettings, get_language::GmailLanguageGet, update_language::GmailLanguageUpdate,
};
use pimalaya_cli::printer::{Message, Printer};

use crate::{account::context::Account, gmail::client::GmailClient};

/// Manage the Gmail display language settings
/// (users.settings.getLanguage / updateLanguage).
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GmailSettingsLanguageCommand {
    Get(GmailSettingsLanguageGetCommand),
    #[command(visible_aliases = ["update"])]
    Set(GmailSettingsLanguageSetCommand),
}

impl GmailSettingsLanguageCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        _account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        match self {
            Self::Get(cmd) => cmd.execute(printer, client),
            Self::Set(cmd) => cmd.execute(printer, client),
        }
    }
}

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

        let text = format!("Display language: {}\n", settings.display_language);

        printer.out(Message::new(text))
    }
}

/// Update the Gmail display language settings.
#[derive(Debug, Parser)]
pub struct GmailSettingsLanguageSetCommand {
    /// Display language tag to set, such as `en` or `fr`.
    #[arg(long, value_name = "LANG")]
    pub display_language: String,
}

impl GmailSettingsLanguageSetCommand {
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
