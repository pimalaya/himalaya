use anyhow::Result;
use clap::{Parser, Subcommand};
use io_gmail::v1::rest::settings::{
    GmailPopSettings, get_pop::GmailPopGet, update_pop::GmailPopUpdate,
};
use pimalaya_cli::printer::{Message, Printer};

use crate::{
    account::context::Account,
    gmail::{
        client::GmailClient,
        settings::convert::{
            access_window_wire, disposition_wire, parse_access_window, parse_disposition,
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

        let mut text = String::new();
        if let Some(access_window) = settings.access_window {
            text.push_str(&format!(
                "Access window: {}\n",
                access_window_wire(access_window)
            ));
        }
        if let Some(disposition) = settings.disposition {
            text.push_str(&format!("Disposition: {}\n", disposition_wire(disposition)));
        }

        printer.out(Message::new(text))
    }
}

/// Update the Gmail POP access settings.
#[derive(Debug, Parser)]
pub struct GmailSettingsPopSetCommand {
    #[arg(long)]
    pub access_window: Option<String>,

    #[arg(long)]
    pub disposition: Option<String>,
}

impl GmailSettingsPopSetCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let access_window = self
            .access_window
            .as_deref()
            .map(parse_access_window)
            .transpose()?;
        let disposition = self
            .disposition
            .as_deref()
            .map(parse_disposition)
            .transpose()?;

        let settings = GmailPopSettings {
            access_window,
            disposition,
        };

        let _out = {
            let c = GmailPopUpdate::new(&client.auth, &client.user_id, settings)?;
            client.run(c)?
        };

        printer.out(Message::new("Gmail POP settings successfully updated"))
    }
}
