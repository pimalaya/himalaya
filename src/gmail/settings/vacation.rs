//! # Gmail vacation settings
//!
//! The `gmail settings vacation` commands, covering
//! `users.settings.getVacation` and its setter.

pub mod get;
pub mod set;

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    gmail::{
        client::GmailClient,
        settings::vacation::{
            get::GmailSettingsVacationGetCommand, set::GmailSettingsVacationSetCommand,
        },
    },
};

/// Manage the Gmail vacation responder settings
/// (users.settings.getVacation / updateVacation).
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GmailSettingsVacationCommand {
    Get(GmailSettingsVacationGetCommand),
    #[command(visible_aliases = ["update"])]
    Set(GmailSettingsVacationSetCommand),
}

impl GmailSettingsVacationCommand {
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
