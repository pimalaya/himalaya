//! # Gmail IMAP set
//!
//! The `gmail settings imap set` command, `users.settings.updateImap`.

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::{get_imap::GmailImapGet, update_imap::GmailImapUpdate};
use pimalaya_cli::printer::{Message, Printer};

use crate::gmail::{
    client::GmailClient,
    settings::convert::{ExpungeBehaviorArg, enabled_flag},
};

/// Update the Gmail IMAP access settings.
///
/// A partial update: the settings are fetched first and only the options
/// passed are changed, so the rest survives. IMAP access itself is
/// toggled with `--enable` and `--disable`, never by accident.
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
    /// Applies the IMAP settings.
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
