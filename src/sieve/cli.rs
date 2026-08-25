use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    sieve::{
        activate::SieveScriptActivateCommand, capability::SieveCapabilityListCommand,
        check::SieveScriptCheckCommand, client::SieveClient,
        deactivate::SieveScriptDeactivateCommand, delete::SieveScriptDeleteCommand,
        get::SieveScriptGetCommand, list::SieveScriptListCommand, put::SieveScriptPutCommand,
        raw::SieveRawCommand, rename::SieveScriptRenameCommand,
    },
};

/// ManageSieve-specific API.
///
/// Gives access to the raw ManageSieve API, always against the account's
/// `[sieve]` block whatever `--backend` says.
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum SieveCommand {
    /// Query server capabilities.
    #[command(alias = "capabilities")]
    Capability(SieveCapabilityListCommand),
    /// List installed scripts.
    List(SieveScriptListCommand),
    /// Download one script.
    Get(SieveScriptGetCommand),
    /// Validate capacity and upload one script.
    Put(SieveScriptPutCommand),
    /// Validate a script without storing it.
    Check(SieveScriptCheckCommand),
    /// Rename one script.
    Rename(SieveScriptRenameCommand),
    /// Delete one script.
    Delete(SieveScriptDeleteCommand),
    /// Activate one script.
    Activate(SieveScriptActivateCommand),
    /// Disable the active script.
    Deactivate(SieveScriptDeactivateCommand),
    /// Send one raw command line.
    Raw(SieveRawCommand),
}

impl SieveCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut SieveClient,
    ) -> Result<()> {
        match self {
            Self::Capability(cmd) => cmd.execute(printer, account, client),
            Self::List(cmd) => cmd.execute(printer, account, client),
            Self::Get(cmd) => cmd.execute(printer, client),
            Self::Put(cmd) => cmd.execute(printer, client),
            Self::Check(cmd) => cmd.execute(printer, client),
            Self::Rename(cmd) => cmd.execute(printer, client),
            Self::Delete(cmd) => cmd.execute(printer, client),
            Self::Activate(cmd) => cmd.execute(printer, client),
            Self::Deactivate(cmd) => cmd.execute(printer, client),
            Self::Raw(cmd) => cmd.execute(printer, client),
        }
    }
}
