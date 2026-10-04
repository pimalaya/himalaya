//! # pimdir message command
//!
//! The `pimdir message` command, dispatching onto its subcommands.

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    pimdir::{
        client::PimdirClient,
        message::{add::PimdirMessageAddCommand, send::PimdirMessageSendCommand},
    },
};

/// Stage a message and get its queue row back.
///
/// The shared `message add` and `message send` print what every backend
/// can say. On a pimdir store the write waits in the queue until the sync
/// engine applies it, and these print the row it waits in, which `pimdir
/// queue show` follows and `pimdir queue cancel` retracts.
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum PimdirMessageCommand {
    Add(PimdirMessageAddCommand),
    Send(PimdirMessageSendCommand),
}

impl PimdirMessageCommand {
    /// Runs the subcommand against the account's pimdir store.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut PimdirClient,
    ) -> Result<()> {
        match self {
            Self::Add(cmd) => cmd.execute(printer, account, client),
            Self::Send(cmd) => cmd.execute(printer, account, client),
        }
    }
}
