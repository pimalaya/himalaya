use std::{
    fs,
    io::{IsTerminal, Read, stdin},
    path::PathBuf,
};

use anyhow::{Context, Result, bail};
use clap::Parser;

/// Sieve source shared by `put` and `check`.
#[derive(Debug, Parser)]
pub struct SieveScriptArg {
    /// Read the script bytes from a file.
    #[arg(long = "script-file", value_name = "PATH", conflicts_with = "script")]
    pub script_file: Option<PathBuf>,
    /// Inline script text, or omit it and pipe the script on stdin.
    #[arg(name = "script", value_name = "SCRIPT", trailing_var_arg = true)]
    pub script: Vec<String>,
}

impl SieveScriptArg {
    /// Resolves the script bytes from the file, the inline argument or
    /// piped stdin, in that order.
    pub fn read(&self) -> Result<Vec<u8>> {
        if let Some(path) = &self.script_file {
            return fs::read(path)
                .with_context(|| format!("Failed to read Sieve script `{}`", path.display()));
        }

        if !self.script.is_empty() {
            return Ok(self.script.join(" ").into_bytes());
        }

        if !stdin().is_terminal() {
            let mut script = Vec::new();
            stdin().read_to_end(&mut script)?;
            return Ok(script);
        }

        bail!("No Sieve script provided: pass SCRIPT, --script-file, or pipe stdin")
    }
}
