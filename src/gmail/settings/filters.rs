//! # Gmail filter settings
//!
//! The `gmail settings filters` commands, covering
//! `users.settings.filters`.

pub mod create;
pub mod delete;
pub mod get;
pub mod list;
pub mod summary;

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    gmail::{
        client::GmailClient,
        settings::filters::{
            create::GmailSettingsFilterCreateCommand, delete::GmailSettingsFilterDeleteCommand,
            get::GmailSettingsFilterGetCommand, list::GmailSettingsFiltersListCommand,
        },
    },
};

/// Manage Gmail filters (users.settings.filters).
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GmailSettingsFiltersCommand {
    List(GmailSettingsFiltersListCommand),
    Get(GmailSettingsFilterGetCommand),
    Create(GmailSettingsFilterCreateCommand),
    #[command(visible_aliases = ["del", "remove", "rm"])]
    Delete(GmailSettingsFilterDeleteCommand),
}

impl GmailSettingsFiltersCommand {
    /// Runs the subcommand against the account's Gmail client.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, account, client),
            Self::Get(cmd) => cmd.execute(printer, client),
            Self::Create(cmd) => cmd.execute(printer, client),
            Self::Delete(cmd) => cmd.execute(printer, client),
        }
    }
}
