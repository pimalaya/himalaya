//! # Gmail send-as settings
//!
//! The `gmail settings sendas` commands, covering
//! `users.settings.sendAs`.

pub mod create;
pub mod delete;
pub mod get;
pub mod list;
pub mod update;
pub mod verify;

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    gmail::{
        client::GmailClient,
        settings::sendas::{
            create::GmailSettingsSendAsCreateCommand, delete::GmailSettingsSendAsDeleteCommand,
            get::GmailSettingsSendAsGetCommand, list::GmailSettingsSendAsListCommand,
            update::GmailSettingsSendAsUpdateCommand, verify::GmailSettingsSendAsVerifyCommand,
        },
    },
};

/// Manage Gmail send-as aliases (settings.sendAs).
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GmailSettingsSendAsCommand {
    List(GmailSettingsSendAsListCommand),
    Get(GmailSettingsSendAsGetCommand),
    Create(GmailSettingsSendAsCreateCommand),
    Update(GmailSettingsSendAsUpdateCommand),
    #[command(visible_aliases = ["del", "remove", "rm"])]
    Delete(GmailSettingsSendAsDeleteCommand),
    Verify(GmailSettingsSendAsVerifyCommand),
}

impl GmailSettingsSendAsCommand {
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
            Self::Update(cmd) => cmd.execute(printer, client),
            Self::Delete(cmd) => cmd.execute(printer, client),
            Self::Verify(cmd) => cmd.execute(printer, client),
        }
    }
}
