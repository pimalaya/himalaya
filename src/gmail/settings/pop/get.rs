//! # Gmail POP get
//!
//! The `gmail settings pop get` command, `users.settings.getPop`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::GmailPopSettings;
use pimalaya_cli::{fields::Fields, printer::Printer};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::gmail::{
    client::GmailClient,
    settings::convert::{access_window_wire, disposition_wire},
};

/// Get the Gmail POP access settings.
#[derive(Debug, Parser)]
pub struct GmailSettingsPopGetCommand;

impl GmailSettingsPopGetCommand {
    /// Fetches the POP settings and prints them.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let settings = client.raw_get(&["settings", "pop"])?;
        printer.out(GmailSettingsPopGetOutput(Value::Object(settings)))
    }
}

impl From<GmailPopSettings> for View {
    fn from(settings: GmailPopSettings) -> Self {
        Self {
            access_window: settings
                .access_window
                .map(|window| access_window_wire(window).to_string()),
            disposition: settings
                .disposition
                .map(|disposition| disposition_wire(disposition).to_string()),
        }
    }
}

/// The fields the text form prints, read from what Gmail sent.
struct View {
    access_window: Option<String>,
    disposition: Option<String>,
}

impl fmt::Display for View {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut fields = Fields::default();

        if let Some(access_window) = &self.access_window {
            fields.push("Access window", access_window);
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
pub(crate) struct GmailSettingsPopGetOutput(
    #[schemars(with = "Map<String, Value>")] pub(crate) Value,
);

impl fmt::Display for GmailSettingsPopGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let settings: GmailPopSettings = serde_json::from_value(self.0.clone()).unwrap_or_default();
        View::from(settings).fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use pimalaya_cli::printer::CapturePrinter;
    use serde_json::json;

    use crate::gmail::{
        client::GmailClient,
        settings::pop::{get::GmailSettingsPopGetCommand, set::GmailSettingsPopSetCommand},
    };

    /// The settings print as Gmail sent them, in its wire spelling, a
    /// property io-gmail does not model kept.
    #[test]
    fn the_settings_print_as_gmail_sent_them() {
        let pop =
            json!({"accessWindow": "FROM_NOW_ON", "disposition": "LEAVE_IN_INBOX", "x-vendor": 1});
        let (mut client, requests) = GmailClient::scripted(std::slice::from_ref(&pop));
        let mut printer = CapturePrinter::default();

        GmailSettingsPopGetCommand
            .execute(&mut printer, &mut client)
            .unwrap();

        assert_eq!(printer.into_single().unwrap(), pop);
        let text = requests.lock().unwrap()[0].to_string();
        assert!(
            text.starts_with("GET /gmail/v1/users/me/settings/pop "),
            "{text}"
        );
    }

    /// A set prints what Gmail answered the update, as it sent it.
    #[test]
    fn a_set_prints_what_gmail_answered() {
        let pop = json!({"accessWindow": "DISABLED", "x-vendor": 1});
        let (mut client, requests) = GmailClient::scripted(std::slice::from_ref(&pop));
        let mut printer = CapturePrinter::default();

        let command = GmailSettingsPopSetCommand {
            access_window: Some(crate::gmail::settings::convert::PopAccessWindowArg::Disabled),
            disposition: None,
        };
        command.execute(&mut printer, &mut client).unwrap();

        assert_eq!(printer.into_single().unwrap(), pop);
        let text = requests.lock().unwrap()[0].to_string();
        assert!(
            text.starts_with("PUT /gmail/v1/users/me/settings/pop "),
            "{text}"
        );
    }
}
