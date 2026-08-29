//! # Gmail history
//!
//! The `gmail history` command family, covering `users.history`.

pub mod list;

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    gmail::{client::GmailClient, history::list::GmailHistoryListCommand},
};

/// Manage the Gmail mailbox history (users.history).
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GmailHistoryCommand {
    List(GmailHistoryListCommand),
}

impl GmailHistoryCommand {
    /// Runs the subcommand against the account's Gmail client.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        _account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, client),
        }
    }
}
