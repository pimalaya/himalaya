//! Shared raw-bytes output helper for commands that return message or
//! attachment content (Gmail, Microsoft Graph).

use std::{
    fs,
    io::{self, IsTerminal, Write},
    path::Path,
};

use anyhow::{Context, Result, bail};
use pimalaya_cli::printer::{Message, Printer};

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
