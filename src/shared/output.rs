//! Shared output helpers for commands that return raw message or
//! attachment content, or paginated listings (Gmail, Microsoft Graph).

use std::{
    fmt, fs,
    io::{self, IsTerminal, Write},
    path::Path,
};

use anyhow::{Context, Result, bail};
use pimalaya_cli::printer::{Message, Printer};
use serde::Serialize;

/// Wraps a renderable listing with an optional pagination cursor so the
/// "next page" hint is part of the command output: a trailing footer
/// line in text mode, an extra `next_page` field in JSON. This keeps
/// the cursor visible to scripts, unlike logging it to stderr.
#[derive(Serialize)]
pub struct Paginated<T> {
    #[serde(flatten)]
    inner: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_page: Option<String>,
}

impl<T> Paginated<T> {
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

/// Writes `bytes` to `output` when given, otherwise to stdout.
///
/// Redirected or piped stdout always receives the verbatim bytes (so
/// `> file.pdf` is byte-exact). To avoid corrupting the display or
/// injecting escape sequences, binary-looking content is refused when
/// stdout is a terminal; text (e.g. a raw RFC 5322 message) still
/// prints interactively.
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

    // NOTE: treat a NUL or a C0 control other than tab/newline/CR as binary;
    // such content can corrupt a terminal or inject escape sequences, so refuse
    // it on a TTY (piped output is unaffected).
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
