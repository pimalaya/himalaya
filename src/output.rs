//! # Output
//!
//! What the commands returning raw content print through.

use std::{
    fs,
    io::{self, IsTerminal},
    path::Path,
};

use anyhow::{Context, Result, bail};
use pimalaya_cli::{
    output::{Bytes, SavedOutput},
    printer::Printer,
};

/// Writes bytes to the given path, or prints them when there is none.
///
/// Verbatim to a redirected stdout, a [`Bytes`] object under `--json`.
/// Binary-looking content is refused on a terminal in text mode, where it
/// would corrupt the display or inject escape sequences.
pub fn write_bytes_or_save(
    printer: &mut impl Printer,
    output: Option<&Path>,
    bytes: Vec<u8>,
    mime: &str,
) -> Result<()> {
    if let Some(path) = output {
        fs::write(path, &bytes).with_context(|| format!("Write `{}` error", path.display()))?;

        return printer.out(SavedOutput {
            path: path.display().to_string(),
            size: bytes.len(),
        });
    }

    if !printer.is_json() && io::stdout().is_terminal() && looks_binary(&bytes) {
        bail!(
            "Refusing to write binary content to the terminal: \
	     redirect stdout or pass --output <PATH>"
        );
    }

    printer.out_bytes(Bytes {
        data: bytes,
        mime: mime.to_owned(),
    })
}

/// Whether `bytes` hold a control character a terminal must be spared: C0
/// other than tab, newline and CR, DEL or C1.
fn looks_binary(bytes: &[u8]) -> bool {
    let is_control = |c: char| c.is_control() && !matches!(c, '\t' | '\n' | '\r');

    match std::str::from_utf8(bytes) {
        Ok(text) => text.chars().any(is_control),
        Err(_) => bytes.iter().any(|&byte| is_control(char::from(byte))),
    }
}

#[cfg(test)]
mod tests {
    use super::looks_binary;

    #[test]
    fn text_is_not_binary() {
        assert!(!looks_binary(b"Hello,\tworld\r\n"));
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
