//! # ManageSieve check
//!
//! The `sieve check` command, RFC 5804 `CHECKSCRIPT`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_managesieve::client::ManagesieveClient as _;
use pimalaya_cli::{printer::Printer, sanitize};
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
            warnings: warning_lines(warnings),
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
    /// The warnings of the server, one per line of its text, `[]` when
    /// it sent none.
    pub(crate) warnings: Vec<String>,
}

impl fmt::Display for SieveScriptOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.name {
            Some(name) => write!(f, "Sieve script successfully uploaded: {name}")?,
            None => write!(f, "Sieve script is valid")?,
        }

        match self.warnings.as_slice() {
            [] => writeln!(f),
            warnings => writeln!(f, " ({})", sanitize::text(&warnings.join("; "))),
        }
    }
}

/// The warnings a server attached to an accepted script, one per line of
/// its text, blank lines dropped.
pub(crate) fn warning_lines(warnings: Option<impl ToString>) -> Vec<String> {
    let Some(warnings) = warnings else {
        return Vec::new();
    };

    warnings
        .to_string()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::sieve::check::{SieveScriptOutput, warning_lines};

    /// The warnings print as a list, one per line the server sent, `[]`
    /// when it sent none, as a receipt carries them.
    #[test]
    fn the_warnings_are_a_list() {
        let output = SieveScriptOutput {
            name: None,
            warnings: warning_lines(Some("line 2: unused\r\nline 5: odd\r\n")),
        };
        let json = serde_json::to_value(&output).unwrap();
        assert_eq!(
            json["warnings"],
            serde_json::json!(["line 2: unused", "line 5: odd"])
        );

        let none = warning_lines(None::<String>);
        assert!(none.is_empty());
    }
}
