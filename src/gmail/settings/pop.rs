use std::fmt;

use anyhow::Result;
use clap::{Parser, Subcommand};
use io_gmail::v1::rest::settings::{
    GmailPopSettings, get_pop::GmailPopGet, update_pop::GmailPopUpdate,
};
use pimalaya_cli::printer::{Message, Printer};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    gmail::{
        client::GmailClient,
        settings::convert::{
            DispositionArg, PopAccessWindowArg, access_window_wire, disposition_wire,
        },
    },
};

/// Manage the Gmail POP access settings
/// (users.settings.getPop / updatePop).
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GmailSettingsPopCommand {
    Get(GmailSettingsPopGetCommand),
    #[command(visible_aliases = ["update"])]
    Set(GmailSettingsPopSetCommand),
}

impl GmailSettingsPopCommand {
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

/// Get the Gmail POP access settings.
#[derive(Debug, Parser)]
pub struct GmailSettingsPopGetCommand;

impl GmailSettingsPopGetCommand {
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

/// Update the Gmail POP access settings.
#[derive(Debug, Parser)]
pub struct GmailSettingsPopSetCommand {
    /// Range of messages made available over POP.
    #[arg(long, value_name = "WINDOW")]
    pub access_window: Option<PopAccessWindowArg>,

    /// Action taken on messages after they are fetched over POP.
    #[arg(long, value_name = "DISPOSITION")]
    pub disposition: Option<DispositionArg>,
}

impl GmailSettingsPopSetCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let settings = GmailPopSettings {
            access_window: self.access_window.map(Into::into),
            disposition: self.disposition.map(Into::into),
        };

        let _out = {
            let c = GmailPopUpdate::new(&client.auth, &client.user_id, settings)?;
            client.run(c)?
        };

        printer.out(Message::new("Gmail POP settings successfully updated"))
    }
}

/// Gmail POP access settings, rendered as aligned text or, under
/// `--json`, as a structured object instead of a wrapped human string.
///
/// The enums keep their Gmail wire spelling, so a value read with `get`
/// is a value `set` accepts back.
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
