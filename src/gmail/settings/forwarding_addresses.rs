pub mod create;
pub mod delete;
pub mod get;
pub mod list;

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    gmail::{
        client::GmailClient,
        settings::forwarding_addresses::{
            create::GmailSettingsForwardingAddressCreateCommand,
            delete::GmailSettingsForwardingAddressDeleteCommand,
            get::GmailSettingsForwardingAddressGetCommand,
            list::GmailSettingsForwardingAddressesListCommand,
        },
    },
};

/// Manage Gmail forwarding addresses (users.settings.forwardingAddresses).
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GmailSettingsForwardingAddressesCommand {
    List(GmailSettingsForwardingAddressesListCommand),
    Get(GmailSettingsForwardingAddressGetCommand),
    Create(GmailSettingsForwardingAddressCreateCommand),
    #[command(visible_aliases = ["del", "remove", "rm"])]
    Delete(GmailSettingsForwardingAddressDeleteCommand),
}

impl GmailSettingsForwardingAddressesCommand {
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
