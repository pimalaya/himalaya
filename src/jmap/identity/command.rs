use anyhow::Result;
use clap::Subcommand;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::jmap::{
    account::JmapAccount,
    identity::{
        create::CreateIdentityCommand, delete::DeleteIdentityCommand, get::GetIdentityCommand,
        update::UpdateIdentityCommand,
    },
};

/// Manage JMAP sender identities.
#[derive(Debug, Subcommand)]
pub enum IdentityCommand {
    /// Fetch identities (Identity/get).
    #[command(aliases = ["lst", "list"])]
    Get(GetIdentityCommand),
    /// Create a new identity (Identity/set).
    #[command(aliases = ["add", "new"])]
    Create(CreateIdentityCommand),
    /// Update an existing identity (Identity/set).
    #[command(alias = "edit")]
    Update(UpdateIdentityCommand),
    /// Delete an identity (Identity/set).
    #[command(aliases = ["remove", "rm"])]
    Delete(DeleteIdentityCommand),
}

impl IdentityCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        match self {
            Self::Get(cmd) => cmd.execute(printer, account),
            Self::Create(cmd) => cmd.execute(printer, account),
            Self::Update(cmd) => cmd.execute(printer, account),
            Self::Delete(cmd) => cmd.execute(printer, account),
        }
    }
}
