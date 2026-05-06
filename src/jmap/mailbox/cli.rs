use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

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
    Get(JmapMailboxGetCommand),
    Query(JmapMailboxQueryCommand),
    #[command(visible_aliases = ["add", "new"])]
    Create(JmapMailboxCreateCommand),
    Update(JmapMailboxUpdateCommand),
    #[command(visible_aliases = ["delete", "del", "remove", "rm"])]
    Destroy(JmapMailboxDestroyCommand),
}

impl JmapMailboxCommand {
    pub fn execute(self, printer: &mut impl Printer, client: JmapClient) -> Result<()> {
        match self {
            Self::Get(cmd) => cmd.execute(printer, client),
            Self::Query(cmd) => cmd.execute(printer, client),
            Self::Create(cmd) => cmd.execute(printer, client),
            Self::Update(cmd) => cmd.execute(printer, client),
            Self::Destroy(cmd) => cmd.execute(printer, client),
        }
    }
}
