//! # Message parse
//!
//! The `message parse` command, printing the `message read` view of a raw
//! message read from a file or stdin, with no account and no
//! configuration.

use std::{
    fmt, fs,
    io::{IsTerminal, Read, stdin},
    path::PathBuf,
};

use anyhow::{Context, Result, bail};
use clap::Parser;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::shared::{
    attachment::download::PartBytes,
    message::{part, read::MessageReadOutput},
};

/// Read a raw message without an account.
///
/// The message is read from the `EML` file, or from the standard input
/// when it is `-` or omitted, and printed exactly as `message read`
/// prints the same bytes: the view under `--json`, the header block and
/// part walk otherwise. No account is resolved and no configuration
/// file is read.
///
/// `--part` writes the decoded bytes of one part to stdout instead, as
/// `attachment download --stdout` does, the ids being those of the
/// view's `parts`.
///
/// A message nested deeper than 8 levels, or holding more than 200 parts
/// or 500 header lines, is refused whole with the `message-too-complex`
/// code, as on `message read`.
#[derive(Debug, Parser)]
pub struct MessageParseCommand {
    /// Path to the raw RFC 5322 message, `-` or nothing for stdin.
    #[arg(value_name = "EML")]
    pub path: Option<PathBuf>,
    /// Write the decoded bytes of this part to stdout, and nothing else.
    ///
    /// Any leaf part the view lists may be named, a body included. With
    /// the global `--json` flag the part comes out as `{id, mime,
    /// filename, size, data}` instead, `data` holding the bytes in
    /// base64.
    #[arg(long, value_name = "PART-ID")]
    pub part: Option<String>,
}

impl MessageParseCommand {
    /// Reads the message and prints its view, or the one part asked.
    pub fn execute(self, printer: &mut impl Printer) -> Result<()> {
        let (raw, source) = self.read()?;

        let Some(id) = self.part else {
            let view = MessageReadOutput::from_raw(&raw)?;
            return printer.out(MessageParseOutput::View(Box::new(view)));
        };

        let message = part::parse(&raw)?;
        let Some(part) = PartBytes::new(&message, &id) else {
            bail!("No part with id {id} in the message read from {source}");
        };

        if printer.is_json() {
            return printer.out(MessageParseOutput::Part(part));
        }

        part.write_stdout()
    }

    /// The raw bytes, as found, and where they were read from.
    fn read(&self) -> Result<(Vec<u8>, String)> {
        let (raw, source) = match self.path.as_deref() {
            Some(path) if path.as_os_str() != "-" => {
                let source = format!("`{}`", path.display());
                let raw = fs::read(path).with_context(|| format!("Failed to read {source}"))?;
                (raw, source)
            }
            path => {
                // NOTE: a bare `message parse` at a prompt would wait for
                // a message nobody announced, where `-` asks for it.
                if path.is_none() && stdin().is_terminal() {
                    bail!("No message provided: pass a file path, or `-` to read stdin");
                }

                let mut raw = Vec::new();
                stdin()
                    .lock()
                    .read_to_end(&mut raw)
                    .context("Failed to read stdin")?;
                (raw, String::from("stdin"))
            }
        };

        if raw.iter().all(u8::is_ascii_whitespace) {
            bail!("The message read from {source} is empty");
        }

        Ok((raw, source))
    }
}

/// The `message parse` output: the view `message read` prints, or under
/// `--part --json` the one part read.
#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
pub enum MessageParseOutput {
    View(Box<MessageReadOutput>),
    Part(PartBytes),
}

impl fmt::Display for MessageParseOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::View(view) => view.fmt(f),
            Self::Part(part) => f.write_str(&part.data),
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::MessageParseCommand;

    fn read(path: &std::path::Path) -> anyhow::Result<Vec<u8>> {
        MessageParseCommand::try_parse_from(["parse", path.to_str().unwrap()])
            .unwrap()
            .read()
            .map(|(raw, _)| raw)
    }

    #[test]
    fn a_file_is_read_as_found() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("m.eml");
        // NOTE: bare newlines and 8-bit bytes stay, as `message read`
        // gets them from the backend.
        std::fs::write(&path, b"Subject: \xe9\n\nbody\n").unwrap();

        assert_eq!(read(&path).unwrap(), b"Subject: \xe9\n\nbody\n");
    }

    #[test]
    fn a_blank_file_is_refused_by_name() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("blank.eml");
        std::fs::write(&path, b" \r\n\t\n").unwrap();

        let err = read(&path).unwrap_err().to_string();
        assert!(err.contains("blank.eml"), "{err}");
        assert!(err.contains("empty"), "{err}");
    }

    #[test]
    fn a_missing_file_is_named() {
        let dir = tempfile::tempdir().unwrap();
        let err = read(&dir.path().join("none.eml")).unwrap_err().to_string();
        assert!(err.contains("none.eml"), "{err}");
    }
}
