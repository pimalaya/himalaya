//! # Gmail POP get
//!
//! The `gmail settings pop get` command, `users.settings.getPop`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::get_pop::GmailPopGet;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::gmail::{
    client::GmailClient,
    settings::convert::{access_window_wire, disposition_wire},
};

/// Get the Gmail POP access settings.
#[derive(Debug, Parser)]
pub struct GmailSettingsPopGetCommand;

impl GmailSettingsPopGetCommand {
    /// Fetches the POP settings and tables them.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let out = {
            let c = GmailPopGet::new(&client.auth, &client.user_id)?;
            client.run(c)?
        };
        let settings = out.response;

        printer.out(GmailSettingsPopGetOutput {
            access_window: settings
                .access_window
                .map(|window| access_window_wire(window).to_string()),
            disposition: settings
                .disposition
                .map(|disposition| disposition_wire(disposition).to_string()),
        })
    }
}

/// The `gmail settings pop get` output.
///
/// The enums keep their Gmail wire spelling, so a value `get` reads is a
/// value `set` accepts back.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct GmailSettingsPopGetOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    access_window: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disposition: Option<String>,
}

impl fmt::Display for GmailSettingsPopGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(access_window) = &self.access_window {
            writeln!(f, "Access window: {access_window}")?;
        }
        if let Some(disposition) = &self.disposition {
            writeln!(f, "Disposition: {disposition}")?;
        }

        Ok(())
    }
}
