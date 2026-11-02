//! # Flag command
//!
//! The `flag` command, dispatching onto its subcommands.

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    capability::{self, Capability},
    email::flag::Flag,
    shared::{
        client::EmailClient,
        flag::{add::FlagAddCommand, remove::FlagRemoveCommand, set::FlagSetCommand},
    },
};

/// Manage flags using the shared API.
///
/// A flag acts like a tag, saying what state or kind a message is in.
#[derive(Debug, Subcommand)]
pub enum FlagCommand {
    #[command(after_long_help = capability::help("flag", "add"))]
    Add(FlagAddCommand),
    #[command(after_long_help = capability::help("flag", "set"))]
    Set(FlagSetCommand),
    #[command(visible_alias = "rm", after_long_help = capability::help("flag", "remove"))]
    Remove(FlagRemoveCommand),
}

impl FlagCommand {
    /// The capabilities the subcommand reaches, one per flag it names,
    /// checked before any connection.
    pub fn capabilities(&self) -> Vec<Capability> {
        let flags = match self {
            Self::Add(cmd) => &cmd.flags.inner,
            Self::Set(cmd) => &cmd.flags.inner,
            Self::Remove(cmd) => &cmd.flags.inner,
        };

        flags
            .iter()
            .filter_map(|flag| Flag::from(flag).capability())
            .collect()
    }

    /// Runs the subcommand against the active account's client.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        match self {
            Self::Add(cmd) => cmd.execute(printer, account, client),
            Self::Set(cmd) => cmd.execute(printer, account, client),
            Self::Remove(cmd) => cmd.execute(printer, account, client),
        }
    }
}
