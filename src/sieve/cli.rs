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
        raw::SieveRawCommand,
    },
};

/// Manage server-side Sieve scripts through RFC 5804 ManageSieve.
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
    /// Activate one script.
    Activate(SieveScriptActivateCommand),
    /// Disable the active script.
    Deactivate(SieveScriptDeactivateCommand),
    /// Delete one script.
    Delete(SieveScriptDeleteCommand),
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
            Self::Activate(cmd) => cmd.execute(printer, client),
            Self::Deactivate(cmd) => cmd.execute(printer, client),
            Self::Delete(cmd) => cmd.execute(printer, client),
            Self::Raw(cmd) => cmd.execute(printer, client),
        }
    }
}
