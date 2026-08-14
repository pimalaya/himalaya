use std::fmt;

use anyhow::Result;
use clap::{Parser, Subcommand};
use io_gmail::v1::rest::settings::{
    get_auto_forwarding::GmailAutoForwardingGet, update_auto_forwarding::GmailAutoForwardingUpdate,
};
use pimalaya_cli::printer::{Message, Printer};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    gmail::{
        client::GmailClient,
        settings::convert::{DispositionArg, disposition_wire, enabled_flag, yes_no},
    },
};

/// Manage the Gmail auto-forwarding settings
/// (users.settings.getAutoForwarding / updateAutoForwarding).
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GmailSettingsAutoForwardingCommand {
    Get(GmailSettingsAutoForwardingGetCommand),
    #[command(visible_aliases = ["update"])]
    Set(GmailSettingsAutoForwardingSetCommand),
}

impl GmailSettingsAutoForwardingCommand {
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

/// Get the Gmail auto-forwarding settings.
#[derive(Debug, Parser)]
pub struct GmailSettingsAutoForwardingGetCommand;

impl GmailSettingsAutoForwardingGetCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let out = {
            let c = GmailAutoForwardingGet::new(&client.auth, &client.user_id)?;
            client.run(c)?
        };
        let settings = out.response;

        printer.out(GmailSettingsAutoForwardingGetOutput {
            enabled: settings.enabled,
            email_address: settings.email_address,
            disposition: settings
                .disposition
                .map(|disposition| disposition_wire(disposition).to_string()),
        })
    }
}

/// Update the Gmail auto-forwarding settings.
///
/// Partial update: the settings are fetched first and only the options
/// you pass are changed, so unspecified fields are preserved.
/// Auto-forwarding is toggled with `--enable` / `--disable` and never
/// by accident.
#[derive(Debug, Parser)]
pub struct GmailSettingsAutoForwardingSetCommand {
    /// Turn auto-forwarding on.
    #[arg(long, conflicts_with = "disable")]
    pub enable: bool,

    /// Turn auto-forwarding off.
    #[arg(long)]
    pub disable: bool,

    /// Address to which incoming messages are forwarded.
    #[arg(long)]
    pub email_address: Option<String>,

    /// Action taken on the original message after it is forwarded.
    #[arg(long, value_name = "DISPOSITION")]
    pub disposition: Option<DispositionArg>,
}

impl GmailSettingsAutoForwardingSetCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let mut settings = {
            let c = GmailAutoForwardingGet::new(&client.auth, &client.user_id)?;
            client.run(c)?.response
        };

        if let Some(enabled) = enabled_flag(self.enable, self.disable) {
            settings.enabled = enabled;
        }
        if let Some(email_address) = self.email_address {
            settings.email_address = Some(email_address);
        }
        if let Some(disposition) = self.disposition {
            settings.disposition = Some(disposition.into());
        }

        let _out = {
            let c = GmailAutoForwardingUpdate::new(&client.auth, &client.user_id, settings)?;
            client.run(c)?
        };

        printer.out(Message::new(
            "Gmail auto-forwarding settings successfully updated",
        ))
    }
}

/// Gmail auto-forwarding settings, rendered as aligned text or, under
/// `--json`, as a structured object instead of a wrapped human string.
///
/// The disposition keeps its Gmail wire spelling, so a value read with
/// `get` is a value `set` accepts back.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct GmailSettingsAutoForwardingGetOutput {
    enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    email_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disposition: Option<String>,
}

impl fmt::Display for GmailSettingsAutoForwardingGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Enabled: {}", yes_no(self.enabled))?;

        if let Some(email_address) = &self.email_address {
            writeln!(f, "Email address: {email_address}")?;
        }
        if let Some(disposition) = &self.disposition {
            writeln!(f, "Disposition: {disposition}")?;
        }

        Ok(())
    }
}
