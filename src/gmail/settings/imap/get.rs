//! # Gmail IMAP get
//!
//! The `gmail settings imap get` command, `users.settings.getImap`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::get_imap::GmailImapGet;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::gmail::{
    client::GmailClient,
    settings::convert::{expunge_behavior_wire, yes_no},
};

/// Get the Gmail IMAP access settings.
#[derive(Debug, Parser)]
pub struct GmailSettingsImapGetCommand;

impl GmailSettingsImapGetCommand {
    /// Fetches the IMAP settings and tables them.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let out = {
            let c = GmailImapGet::new(&client.auth, &client.user_id)?;
            client.run(c)?
        };
        let settings = out.response;

        printer.out(GmailSettingsImapGetOutput {
            enabled: settings.enabled,
            auto_expunge: settings.auto_expunge,
            expunge_behavior: settings
                .expunge_behavior
                .map(|behavior| expunge_behavior_wire(behavior).to_string()),
            max_folder_size: settings.max_folder_size,
        })
    }
}

/// The `gmail settings imap get` output.
///
/// A boolean stays a boolean in JSON where the text spells it yes or no,
/// and the expunge behaviour keeps its Gmail wire spelling, so a value
/// `get` reads is a value `set` accepts back.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct GmailSettingsImapGetOutput {
    enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    auto_expunge: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expunge_behavior: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_folder_size: Option<u32>,
}

impl fmt::Display for GmailSettingsImapGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Enabled: {}", yes_no(self.enabled))?;

        if let Some(auto_expunge) = self.auto_expunge {
            writeln!(f, "Auto expunge: {}", yes_no(auto_expunge))?;
        }
        if let Some(behavior) = &self.expunge_behavior {
            writeln!(f, "Expunge behavior: {behavior}")?;
        }
        if let Some(size) = self.max_folder_size {
            writeln!(f, "Max folder size: {size}")?;
        }

        Ok(())
    }
}
