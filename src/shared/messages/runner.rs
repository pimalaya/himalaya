//! Spawns user-defined composer and reader commands.
//!
//! A composer/reader is just a shell command line invoked via
//! `sh -c`. Himalaya pipes source MIME bytes (empty for new messages)
//! into the child's stdin, captures stdout (the produced MIME draft
//! or the interpreted text), and inherits stderr so the spawned
//! command can prompt the user or print errors directly to the
//! terminal. TUI composers that need interactive input can re-open
//! `/dev/tty` once they've consumed stdin — standard Unix practice.

use std::{
    collections::HashMap,
    io::Write,
    process::{Command, Stdio},
};

use anyhow::{anyhow, bail, Result};

use crate::config::{ComposerConfig, ReaderConfig};

/// Resolves a composer entry to its shell command line. When `name`
/// is given, looks up the corresponding entry and bails if missing.
/// When `name` is `None`, returns the entry with `default = true`,
/// or bails with a hint if no default is set.
pub fn resolve_composer<'a>(
    composers: &'a HashMap<String, ComposerConfig>,
    name: Option<&str>,
) -> Result<&'a str> {
    match name {
        Some(name) => match composers.get(name) {
            Some(entry) => Ok(entry.command.as_str()),
            None => bail!("no composer named `{name}` in [message.composer]"),
        },
        None => default_composer(composers).map(|entry| entry.command.as_str()),
    }
}

/// Same as [`resolve_composer`] but for readers.
pub fn resolve_reader<'a>(
    readers: &'a HashMap<String, ReaderConfig>,
    name: Option<&str>,
) -> Result<&'a str> {
    match name {
        Some(name) => match readers.get(name) {
            Some(entry) => Ok(entry.command.as_str()),
            None => bail!("no reader named `{name}` in [message.reader]"),
        },
        None => default_reader(readers).map(|entry| entry.command.as_str()),
    }
}

fn default_composer(composers: &HashMap<String, ComposerConfig>) -> Result<&ComposerConfig> {
    composers.values().find(|c| c.default).ok_or_else(|| {
        anyhow!(
            "no composer specified and no default in [message.composer.*]; \
                 pass a <name> or set `default = true` on one entry"
        )
    })
}

fn default_reader(readers: &HashMap<String, ReaderConfig>) -> Result<&ReaderConfig> {
    readers.values().find(|c| c.default).ok_or_else(|| {
        anyhow!(
            "no reader specified and no default in [message.reader.*]; \
             pass a <name> or set `default = true` on one entry"
        )
    })
}

/// Spawns `command` through `sh -c`, writes `stdin_bytes` to its
/// stdin, and returns the captured stdout bytes. Stderr is inherited.
/// Bails on a non-zero exit status.
pub fn run(command: &str, stdin_bytes: &[u8]) -> Result<Vec<u8>> {
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|err| anyhow!("spawn `{command}`: {err}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(stdin_bytes)
            .map_err(|err| anyhow!("write stdin to `{command}`: {err}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|err| anyhow!("wait `{command}`: {err}"))?;

    if !output.status.success() {
        bail!(
            "`{command}` exited with status {}",
            output
                .status
                .code()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "?".to_string())
        );
    }

    Ok(output.stdout)
}
