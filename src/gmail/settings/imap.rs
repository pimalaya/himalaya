use std::fmt;

use anyhow::Result;
use clap::{Parser, Subcommand};
use io_gmail::v1::rest::settings::{get_imap::GmailImapGet, update_imap::GmailImapUpdate};
use pimalaya_cli::printer::{Message, Printer};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    gmail::{
        client::GmailClient,
        settings::convert::{ExpungeBehaviorArg, enabled_flag, expunge_behavior_wire, yes_no},
    },
};

/// Manage the Gmail IMAP access settings
/// (users.settings.getImap / updateImap).
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GmailSettingsImapCommand {
    Get(GmailSettingsImapGetCommand),
    #[command(visible_aliases = ["update"])]
    Set(GmailSettingsImapSetCommand),
}

impl GmailSettingsImapCommand {
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

/// Get the Gmail IMAP access settings.
#[derive(Debug, Parser)]
pub struct GmailSettingsImapGetCommand;

impl GmailSettingsImapGetCommand {
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

/// Update the Gmail IMAP access settings.
///
/// Partial update: the settings are fetched first and only the options
/// you pass are changed, so unspecified fields are preserved. IMAP
/// access is toggled with `--enable` / `--disable` and never by
/// accident.
#[derive(Debug, Parser)]
pub struct GmailSettingsImapSetCommand {
    /// Turn IMAP access on.
    #[arg(long, conflicts_with = "disable")]
    pub enable: bool,

    /// Turn IMAP access off.
    #[arg(long)]
    pub disable: bool,

    /// Auto-expunge messages when their last label is removed.
    #[arg(long)]
    pub auto_expunge: Option<bool>,

    /// Action taken on messages marked deleted in IMAP.
    #[arg(long, value_name = "BEHAVIOR")]
    pub expunge_behavior: Option<ExpungeBehaviorArg>,

    /// Maximum number of messages exposed in an IMAP folder.
    #[arg(long)]
    pub max_folder_size: Option<u32>,
}

impl GmailSettingsImapSetCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let mut settings = {
            let c = GmailImapGet::new(&client.auth, &client.user_id)?;
            client.run(c)?.response
        };

        if let Some(enabled) = enabled_flag(self.enable, self.disable) {
            settings.enabled = enabled;
        }
        if let Some(auto_expunge) = self.auto_expunge {
            settings.auto_expunge = Some(auto_expunge);
        }
        if let Some(behavior) = self.expunge_behavior {
            settings.expunge_behavior = Some(behavior.into());
        }
        if let Some(size) = self.max_folder_size {
            settings.max_folder_size = Some(size);
        }

        let _out = {
            let c = GmailImapUpdate::new(&client.auth, &client.user_id, settings)?;
            client.run(c)?
        };

        printer.out(Message::new("Gmail IMAP settings successfully updated"))
    }
}

/// Gmail IMAP access settings, rendered as aligned text or, under
/// `--json`, as a structured object instead of a wrapped human string.
///
/// The booleans stay booleans in JSON, where the text rendering spells
/// them yes and no; the expunge behavior keeps its Gmail wire spelling,
/// so a value read with `get` is a value `set` accepts back.
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
