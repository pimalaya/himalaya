use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::account::context::Account;
use crate::jmap::{
    client::JmapClient,
    identity::{
        create::JmapIdentityCreateCommand, delete::JmapIdentityDeleteCommand,
        get::JmapIdentityGetCommand, update::JmapIdentityUpdateCommand,
    },
};

/// Manage JMAP sender identities.
#[derive(Debug, Subcommand)]
pub enum JmapIdentityCommand {
    /// Fetch identities (Identity/get).
    #[command(aliases = ["lst", "list"])]
    Get(JmapIdentityGetCommand),
    /// Create a new identity (Identity/set).
    #[command(aliases = ["add", "new"])]
    Create(JmapIdentityCreateCommand),
    /// Update an existing identity (Identity/set).
    #[command(alias = "edit")]
    Update(JmapIdentityUpdateCommand),
    /// Delete an identity (Identity/set).
    #[command(aliases = ["remove", "rm"])]
    Delete(JmapIdentityDeleteCommand),
}

impl JmapIdentityCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut JmapClient,
    ) -> Result<()> {
        match self {
            Self::Get(cmd) => cmd.execute(printer, account, client),
            Self::Create(cmd) => cmd.execute(printer, client),
            Self::Update(cmd) => cmd.execute(printer, client),
            Self::Delete(cmd) => cmd.execute(printer, client),
        }
    }
}
