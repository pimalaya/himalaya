use anyhow::Result;
use clap::{Parser, Subcommand};
use io_gmail::v1::rest::settings::{
    GmailImapSettings, get_imap::GmailImapGet, update_imap::GmailImapUpdate,
};
use pimalaya_cli::printer::{Message, Printer};

use crate::{
    account::context::Account,
    gmail::{
        client::GmailClient,
        settings::convert::{ExpungeBehaviorArg, expunge_behavior_wire},
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

        let mut text = String::new();
        text.push_str(&format!(
            "Enabled: {}\n",
            if settings.enabled { "yes" } else { "no" }
        ));
        if let Some(auto_expunge) = settings.auto_expunge {
            text.push_str(&format!(
                "Auto expunge: {}\n",
                if auto_expunge { "yes" } else { "no" }
            ));
        }
        if let Some(behavior) = settings.expunge_behavior {
            text.push_str(&format!(
                "Expunge behavior: {}\n",
                expunge_behavior_wire(behavior)
            ));
        }
        if let Some(size) = settings.max_folder_size {
            text.push_str(&format!("Max folder size: {size}\n"));
        }

        printer.out(Message::new(text))
    }
}

/// Update the Gmail IMAP access settings.
#[derive(Debug, Parser)]
pub struct GmailSettingsImapSetCommand {
    #[arg(long)]
    pub enable: bool,

    #[arg(long)]
    pub auto_expunge: Option<bool>,

    #[arg(long, value_name = "BEHAVIOR")]
    pub expunge_behavior: Option<ExpungeBehaviorArg>,

    #[arg(long)]
    pub max_folder_size: Option<u32>,
}

impl GmailSettingsImapSetCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let settings = GmailImapSettings {
            enabled: self.enable,
            auto_expunge: self.auto_expunge,
            expunge_behavior: self.expunge_behavior.map(Into::into),
            max_folder_size: self.max_folder_size,
        };

        let _out = {
            let c = GmailImapUpdate::new(&client.auth, &client.user_id, settings)?;
            client.run(c)?
        };

        printer.out(Message::new("Gmail IMAP settings successfully updated"))
    }
}
