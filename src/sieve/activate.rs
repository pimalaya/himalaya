//! # ManageSieve activate
//!
//! The `sieve activate` command, RFC 5804 `SETACTIVE`.

use anyhow::Result;
use clap::Parser;
use io_managesieve::client::ManagesieveClient as _;
use pimalaya_cli::printer::{Message, Printer};

use crate::sieve::client::SieveClient;

/// Make one server-side Sieve script active.
#[derive(Debug, Parser)]
pub struct SieveScriptActivateCommand {
    /// The script name.
    #[arg(value_name = "NAME")]
    pub name: String,
}

impl SieveScriptActivateCommand {
    /// Makes the named script the active one.
    pub fn execute(self, printer: &mut impl Printer, client: &mut SieveClient) -> Result<()> {
        client.activate_script(Some(self.name))?;
        printer.out(Message::new("Sieve script successfully activated"))
    }
}
