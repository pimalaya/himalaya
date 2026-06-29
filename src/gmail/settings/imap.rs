use anyhow::Result;
use clap::{Parser, Subcommand};
use io_gmail::v1::rest::settings::{get_imap::GmailImapGet, update_imap::GmailImapUpdate};
use pimalaya_cli::printer::{Message, Printer};

use crate::{
    account::context::Account,
    gmail::{
        client::GmailClient,
        settings::convert::{ExpungeBehaviorArg, enabled_flag, expunge_behavior_wire},
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
