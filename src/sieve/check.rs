//! # ManageSieve check
//!
//! The `sieve check` command, RFC 5804 `CHECKSCRIPT`.

use anyhow::Result;
use clap::Parser;
use io_managesieve::client::ManagesieveClient as _;
use pimalaya_cli::printer::{Message, Printer};

use crate::sieve::{client::SieveClient, script::SieveScriptArg};

/// Validate a Sieve script on the server without storing it.
#[derive(Debug, Parser)]
pub struct SieveScriptCheckCommand {
    #[command(flatten)]
    pub script: SieveScriptArg,
}

impl SieveScriptCheckCommand {
    /// Asks the server to validate the script without storing it.
    pub fn execute(self, printer: &mut impl Printer, client: &mut SieveClient) -> Result<()> {
        let message = match client.check_script(self.script.read()?)? {
            Some(warnings) => format!("Sieve script is valid: {warnings}"),
            None => String::from("Sieve script is valid"),
        };

        printer.out(Message::new(message))
    }
}
