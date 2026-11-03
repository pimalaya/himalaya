//! # ManageSieve check
//!
//! The `sieve check` command, RFC 5804 `CHECKSCRIPT`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_managesieve::client::ManagesieveClient as _;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

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
        let warnings = client.check_script(self.script.read()?)?;

        printer.out(SieveScriptOutput {
            name: None,
            warnings: warnings.map(|warnings| warnings.to_string()),
        })
    }
}

/// What `sieve check` and `sieve put` print: the script, and the
/// warnings the server attached to accepting it.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SieveScriptOutput {
    /// The name of the script uploaded, `null` for a check.
    pub(crate) name: Option<String>,
    /// The warnings of the server, `null` when it sent none.
    pub(crate) warnings: Option<String>,
}

impl fmt::Display for SieveScriptOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.name {
            Some(name) => write!(f, "Sieve script successfully uploaded: {name}")?,
            None => write!(f, "Sieve script is valid")?,
        }

        match &self.warnings {
            Some(warnings) => writeln!(f, " ({warnings})"),
            None => writeln!(f),
        }
    }
}
