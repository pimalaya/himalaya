//! # Gmail auto-forwarding settings
//!
//! The `gmail settings autoforwarding` commands, covering
//! `users.settings.getAutoForwarding` and its setter.

pub mod get;
pub mod set;

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    gmail::{
        client::GmailClient,
        settings::autoforwarding::{
            get::GmailSettingsAutoForwardingGetCommand, set::GmailSettingsAutoForwardingSetCommand,
        },
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
    /// Runs the subcommand against the account's Gmail client.
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
