use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::account::context::Account;
use crate::jmap::{
    client::JmapClient,
    mailbox::{
        create::JmapMailboxCreateCommand, destroy::JmapMailboxDestroyCommand,
        get::JmapMailboxGetCommand, query::JmapMailboxQueryCommand,
        update::JmapMailboxUpdateCommand,
    },
};

/// Manage JMAP mailboxes.
#[derive(Debug, Subcommand)]
pub enum JmapMailboxCommand {
    /// Get JMAP mailboxes by ID (Mailbox/get).
    Get(JmapMailboxGetCommand),
    /// Query JMAP mailboxes (Mailbox/query + Mailbox/get).
    Query(JmapMailboxQueryCommand),
    /// Create a JMAP mailbox.
    #[command(visible_aliases = ["add", "new"])]
    Create(JmapMailboxCreateCommand),
    /// Update a JMAP mailbox.
    Update(JmapMailboxUpdateCommand),
    /// Delete a JMAP mailbox.
    #[command(visible_aliases = ["delete", "del", "remove", "rm"])]
    Destroy(JmapMailboxDestroyCommand),
}

impl JmapMailboxCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut JmapClient,
    ) -> Result<()> {
        match self {
            Self::Get(cmd) => cmd.execute(printer, account, client),
            Self::Query(cmd) => cmd.execute(printer, account, client),
            Self::Create(cmd) => cmd.execute(printer, client),
            Self::Update(cmd) => cmd.execute(printer, client),
            Self::Destroy(cmd) => cmd.execute(printer, client),
        }
    }
}
