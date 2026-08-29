//! # Gmail language settings
//!
//! The `gmail settings language` commands, covering
//! `users.settings.getLanguage` and its setter.

pub mod get;
pub mod set;

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    gmail::{
        client::GmailClient,
        settings::language::{
            get::GmailSettingsLanguageGetCommand, set::GmailSettingsLanguageSetCommand,
        },
    },
};

/// Manage the Gmail display language settings
/// (users.settings.getLanguage / updateLanguage).
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GmailSettingsLanguageCommand {
    Get(GmailSettingsLanguageGetCommand),
    #[command(visible_aliases = ["update"])]
    Set(GmailSettingsLanguageSetCommand),
}

impl GmailSettingsLanguageCommand {
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
