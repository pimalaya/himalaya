//! # ManageSieve put
//!
//! The `sieve put` command, RFC 5804 `PUTSCRIPT`.

use anyhow::Result;
use clap::Parser;
use io_managesieve::client::ManagesieveClient as _;
use pimalaya_cli::printer::Printer;

use crate::sieve::{check::SieveScriptOutput, client::SieveClient, script::SieveScriptArg};

/// Validate capacity and upload one server-side Sieve script.
///
/// The server compiles the script, so a failure names the line at fault
/// and a warning is printed next to the success.
#[derive(Debug, Parser)]
pub struct SieveScriptPutCommand {
    /// The script name.
    #[arg(value_name = "NAME")]
    pub name: String,
    #[command(flatten)]
    pub script: SieveScriptArg,
}

impl SieveScriptPutCommand {
    /// Uploads the script under the given name.
    pub fn execute(self, printer: &mut impl Printer, client: &mut SieveClient) -> Result<()> {
        let script = self.script.read()?;

        client.have_space(self.name.clone(), script.len() as u32)?;

        let warnings = client.put_script(self.name.clone(), script)?;

        printer.out(SieveScriptOutput {
            name: Some(self.name),
            warnings: warnings.map(|warnings| warnings.to_string()),
        })
    }
}
