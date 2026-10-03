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

    if stdout.is_terminal() && looks_binary(bytes) {
        bail!(
            "Refusing to write binary content to the terminal: \
	     redirect stdout or pass --output <PATH>"
        );
    }

    stdout.write_all(bytes).context("Write to stdout error")?;
    stdout.flush().context("Flush stdout error")?;

    Ok(())
}

/// Whether `bytes` read as binary, which is what a terminal must be
/// spared: a control character other than tab, newline and CR, that is
/// NUL and the rest of C0, DEL and C1.
fn looks_binary(bytes: &[u8]) -> bool {
    let is_control = |c: char| c.is_control() && !matches!(c, '\t' | '\n' | '\r');

    match std::str::from_utf8(bytes) {
        Ok(text) => text.chars().any(is_control),
        // NOTE: bytes that are not UTF-8 are read as Latin-1, where
        // 0x80..=0x9f are the C1 controls.
        Err(_) => bytes.iter().any(|&byte| is_control(char::from(byte))),
    }
}

#[cfg(test)]
mod tests {
    use super::looks_binary;

    #[test]
    fn text_is_not_binary() {
        assert!(!looks_binary(b"Hello,\tworld\r\n"));
        // UTF-8 continuation bytes in 0x80..=0x9f are not C1 controls.
        assert!(!looks_binary("Příliš žluťoučký kůň\n".as_bytes()));
    }

    #[test]
    fn control_characters_are_binary() {
        assert!(looks_binary(b"\x00"));
        assert!(looks_binary(b"a\x1b[2Jb"));
        assert!(looks_binary(b"a\x7fb"));
        assert!(looks_binary("a\u{9b}2Jb".as_bytes()));
        assert!(looks_binary(b"a\x9b2Jb"));
    }
}
