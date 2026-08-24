use std::fmt;

use anyhow::Result;
use clap::Parser;
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
    pub fn execute(self, printer: &mut impl Printer, client: &mut SieveClient) -> Result<()> {
        let script = client.get_script(&self.name)?;
        let script = String::from_utf8(script)
            .map_err(|_| anyhow::anyhow!("ManageSieve script is not valid UTF-8"))?;
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
    pub name: String,
    pub script: String,
}

impl fmt::Display for SieveScriptOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.script)
    }
}
