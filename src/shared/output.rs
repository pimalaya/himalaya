//! # Output
//!
//! What the commands returning raw content or a paginated listing print
//! through.

use std::{
    fmt, fs,
    io::{self, IsTerminal, Write},
    path::Path,
};

use anyhow::{Context, Result, bail};
use pimalaya_cli::printer::{Message, Printer};
use schemars::JsonSchema;
use serde::Serialize;

/// A listing plus the cursor of its next page.
///
/// The cursor is part of the output, a footer line in text and a
/// `next_page` field in JSON, so a script reads it. Logging it to stderr
/// would not.
#[derive(Serialize, JsonSchema)]
pub struct Paginated<T> {
    #[serde(flatten)]
    inner: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_page: Option<String>,
}

impl<T> Paginated<T> {
    /// Wraps a listing and the cursor its backend returned.
    pub fn new(inner: T, next_page: Option<String>) -> Self {
        Self { inner, next_page }
    }
}

impl<T: fmt::Display> fmt::Display for Paginated<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)?;

        if let Some(cursor) = &self.next_page {
            writeln!(f, "Next page: {cursor}")?;
        }

        Ok(())
    }
}

/// Writes bytes to the given path, or to stdout when there is none.
///
/// A redirected stdout receives the bytes verbatim, so a saved file is
/// byte-exact. Binary-looking content is refused on a terminal, where it
/// would corrupt the display or inject escape sequences.
pub fn write_bytes_or_save(
    printer: &mut impl Printer,
    output: Option<&Path>,
    bytes: &[u8],
) -> Result<()> {
    if let Some(path) = output {
        fs::write(path, bytes).with_context(|| format!("Write `{}` error", path.display()))?;

        return printer.out(Message::new(format!(
            "Saved {} bytes to {}",
            bytes.len(),
            path.display()
        )));
    }

    let mut stdout = io::stdout();

    // NOTE: a NUL or a C0 control other than tab, newline and CR reads as
    // binary, which is what a terminal must be spared.
    let looks_binary = bytes
        .iter()
        .any(|&byte| byte == 0 || (byte < 0x20 && !matches!(byte, b'\t' | b'\n' | b'\r')));

    if stdout.is_terminal() && looks_binary {
        bail!(
            "Refusing to write binary content to the terminal: \
	     redirect stdout or pass --output <PATH>"
        );
    }

    stdout.write_all(bytes).context("Write to stdout error")?;
    stdout.flush().context("Flush stdout error")?;

    Ok(())
}
