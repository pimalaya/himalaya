//! # Gmail profile
//!
//! The `gmail profile` command family, covering `users.getProfile`.

pub mod get;

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    gmail::{client::GmailClient, profile::get::GmailProfileGetCommand},
};

/// Manage the Gmail user profile (users.getProfile).
#[derive(Debug, Subcommand)]
pub enum GmailProfileCommand {
    Get(GmailProfileGetCommand),
}

impl GmailProfileCommand {
    /// Runs the subcommand against the account's Gmail client.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        _account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        match self {
            Self::Get(cmd) => cmd.execute(printer, client),
        }
    }
}
