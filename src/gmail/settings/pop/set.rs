//! # Gmail POP set
//!
//! The `gmail settings pop set` command, `users.settings.updatePop`.

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::{GmailPopSettings, update_pop::GmailPopUpdate};
use pimalaya_cli::printer::{Message, Printer};

use crate::gmail::{
    client::GmailClient,
    settings::convert::{DispositionArg, PopAccessWindowArg},
};

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
    /// Applies the POP settings.
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
