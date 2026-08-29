//! # ManageSieve get
//!
//! The `sieve get` command, RFC 5804 `GETSCRIPT`.

use std::fmt;

use anyhow::{Result, anyhow};
use clap::Parser;
use io_managesieve::client::ManagesieveClient as _;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::sieve::client::SieveClient;

/// Download and print one server-side Sieve script.
#[derive(Debug, Parser)]
pub struct SieveScriptGetCommand {
    /// The script name.
    #[arg(value_name = "NAME")]
    pub name: String,
}

impl SieveScriptGetCommand {
    /// Downloads the named script and prints it.
    pub fn execute(self, printer: &mut impl Printer, client: &mut SieveClient) -> Result<()> {
        let script = client.get_script(self.name.clone())?;
        let script = String::from_utf8(script)
            .map_err(|_| anyhow!("Sieve script `{}` is not valid UTF-8", self.name))?;

        printer.out(SieveScriptOutput {
            name: self.name,
            script,
        })
    }
}

/// Structured output for `sieve get`.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct SieveScriptOutput {
    /// The script name.
    pub name: String,
    /// The script source.
    pub script: String,
}

impl fmt::Display for SieveScriptOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.script)
    }
}
