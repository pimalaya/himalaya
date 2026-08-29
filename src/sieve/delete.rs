//! # ManageSieve delete
//!
//! The `sieve delete` command, RFC 5804 `DELETESCRIPT`.

use anyhow::Result;
use clap::Parser;
use io_managesieve::client::ManagesieveClient as _;
use pimalaya_cli::printer::{Message, Printer};

use crate::sieve::client::SieveClient;

/// Delete one server-side Sieve script.
///
/// A server refuses to delete the active script, so deactivate it
/// first.
#[derive(Debug, Parser)]
pub struct SieveScriptDeleteCommand {
    /// The script name.
    #[arg(value_name = "NAME")]
    pub name: String,
}

impl SieveScriptDeleteCommand {
    /// Deletes the named script.
    pub fn execute(self, printer: &mut impl Printer, client: &mut SieveClient) -> Result<()> {
        client.delete_script(self.name)?;
        printer.out(Message::new("Sieve script successfully deleted"))
    }
}
