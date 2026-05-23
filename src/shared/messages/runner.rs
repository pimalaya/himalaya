// This file is part of Himalaya, a CLI to manage emails.
//
// Copyright (C) 2022-2026 soywod <pimalaya.org@posteo.net>
//
// This program is free software: you can redistribute it and/or modify it under
// the terms of the GNU Affero General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option) any
// later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more
// details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

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
    io::Write,
    process::{Command, Stdio},
};

use anyhow::{Result, anyhow, bail};

/// Spawns `command`, writes `stdin_bytes` to its
/// stdin, and returns the captured stdout bytes.
/// Stderr is inherited.
/// Bails on a non-zero exit status.
pub fn run(command: &mut Command, stdin_bytes: &[u8]) -> Result<Vec<u8>> {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|err| anyhow!("spawn `{command:?}`: {err}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(stdin_bytes)
            .map_err(|err| anyhow!("write stdin to `{command:?}`: {err}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|err| anyhow!("wait `{command:?}`: {err}"))?;

    if !output.status.success() {
        bail!(
            "`{command:?}` exited with status {}",
            output
                .status
                .code()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "?".to_string())
        );
    }

    Ok(output.stdout)
}
