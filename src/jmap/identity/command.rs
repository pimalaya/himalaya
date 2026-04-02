use anyhow::Result;
use clap::Subcommand;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::jmap::{
    account::JmapAccount,
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
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        match self {
            Self::Get(cmd) => cmd.execute(printer, account),
            Self::Create(cmd) => cmd.execute(printer, account),
            Self::Update(cmd) => cmd.execute(printer, account),
            Self::Delete(cmd) => cmd.execute(printer, account),
        }
    }
}
