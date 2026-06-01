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

//! Reusable clap arg for raw RFC 5322 message input.
//!
//! Ported verbatim from `mml::cli::args::MessageArg` so every
//! message-source command (shared `messages add`/`send`, per-protocol
//! `imap message save`, `maildir message save`, `jmap email import`,
//! `smtp message send`) accepts the same three forms: a file path, an
//! inline raw message, or stdin.

use std::{
    fs,
    io::{IsTerminal, stdin},
};

use anyhow::bail;
use clap::Parser;
use pimalaya_cli::clap::parsers::path_parser;

/// Trailing positional that resolves to a raw RFC 5322 message.
///
/// Resolution order:
///
/// 1. When the positional arg is non-empty: join the tokens with a
///    space, replace `\r` / `\n` literals with `\r\n`, and treat the
///    result as a path. If the path parses and the file is readable,
///    return its contents; otherwise treat the joined value as the
///    raw message verbatim.
/// 2. Otherwise, when stdin is piped, return stdin lines joined with
///    `\r\n`.
/// 3. Otherwise, bail.
#[derive(Debug, Parser)]
pub struct MessageArg {
    /// Can be a path to a file, raw message contents or nothing if
    /// piped via standard input.
    #[arg(name = "message-raw", value_name = "MESSAGE", raw = true)]
    pub raw: Vec<String>,
}

impl MessageArg {
    pub fn parse(&self) -> anyhow::Result<String> {
        if !self.raw.is_empty() {
            let mime = self.raw.join(" ").replace("\\r", "").replace("\\n", "\r\n");

            let Ok(path) = path_parser(&mime) else {
                return Ok(mime);
            };

            let Ok(mime) = fs::read_to_string(path) else {
                return Ok(mime);
            };

            return Ok(mime);
        }

        if !stdin().is_terminal() {
            let lines: Vec<_> = stdin().lines().map_while(Result::ok).collect();
            return Ok(lines.join("\r\n"));
        }

        bail!("Message cannot be empty");
    }
}
