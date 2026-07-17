use std::path::PathBuf;

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::{check::AccountCheckCommand, list::AccountListCommand},
    backend::Backend,
};

/// Manage accounts defined in the TOML configuration file.
///
/// An account is a named group of backend settings (imap, jmap,
/// maildir, smtp). Use these subcommands to inspect them or validate
/// their connection. To create a new account, run bare `himalaya`.
#[derive(Debug, Subcommand)]
pub enum AccountCommand {
    #[command(visible_alias = "ls")]
    List(AccountListCommand),
    Check(AccountCheckCommand),
}

impl AccountCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        config_paths: &[PathBuf],
        account_name: Option<&str>,
        backend: Backend,
    ) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, config_paths),
            Self::Check(cmd) => cmd.execute(printer, config_paths, account_name, backend),
        }
    }
}
