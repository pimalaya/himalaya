//! # Microsoft Graph profile command
//!
//! The `msgraph profile` command, dispatching onto its subcommands.

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    msgraph::{client::MsgraphClient, profile::get::MsgraphProfileGetCommand},
};

/// Manage the Microsoft Graph signed-in user (`GET /me`).
#[derive(Debug, Subcommand)]
pub enum MsgraphProfileCommand {
    Get(MsgraphProfileGetCommand),
}

impl MsgraphProfileCommand {
    /// Runs the subcommand against the account's Graph client.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        _account: &mut Account,
        client: &mut MsgraphClient,
    ) -> Result<()> {
        match self {
            Self::Get(cmd) => cmd.execute(printer, client),
        }
    }
}
