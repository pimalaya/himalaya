//! # ManageSieve rename
//!
//! The `sieve rename` command, RFC 5804 `RENAMESCRIPT`.

use anyhow::Result;
use clap::Parser;
use io_managesieve::client::ManagesieveClient as _;
use pimalaya_cli::printer::Printer;

use crate::{shared::receipt::IdReceipt, sieve::client::SieveClient};

/// Rename one server-side Sieve script.
///
/// Renaming the active script keeps it active. Servers predating RFC
/// 5804 do not carry the command and reject it.
#[derive(Debug, Parser)]
pub struct SieveScriptRenameCommand {
    /// The name the script has now.
    #[arg(value_name = "NAME")]
    pub name: String,
    /// The name it should have.
    #[arg(value_name = "NEW-NAME")]
    pub new_name: String,
}

impl SieveScriptRenameCommand {
    /// Renames the named script.
    pub fn execute(self, printer: &mut impl Printer, client: &mut SieveClient) -> Result<()> {
        client.rename_script(self.name, self.new_name.clone())?;

        printer.out(IdReceipt {
            id: self.new_name,
            done: "Sieve script successfully renamed",
        })
    }
}
