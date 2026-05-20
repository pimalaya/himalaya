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

use std::{
    io::{stdin, BufRead, IsTerminal},
    path::PathBuf,
};

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::shared::{client::EmailClient, messages::output::extract_envelope};

/// Send a message via the active account.
///
/// Routes through SMTP or JMAP depending on the account's configured
/// outgoing backend. The envelope sender is taken from the `From:`
/// header and recipients are collected from `To:` / `Cc:` / `Bcc:`.
///
/// Source priority: `--file <PATH>` (read from file), otherwise stdin
/// when piped, otherwise the positional `<MESSAGE>` args joined with
/// CRLF.
#[derive(Debug, Parser)]
pub struct MessageSendCommand {
    /// Read the raw message from this file instead of stdin or the
    /// positional argument.
    #[arg(long = "file", value_name = "PATH")]
    pub file: Option<PathBuf>,

    /// The raw message, including headers and body.
    #[arg(trailing_var_arg = true)]
    #[arg(name = "message", value_name = "MESSAGE")]
    pub message: Vec<String>,
}

impl MessageSendCommand {
    pub fn execute(self, printer: &mut impl Printer, mut client: EmailClient) -> Result<()> {
        let raw: Vec<u8> = if let Some(path) = self.file.as_deref() {
            std::fs::read(path)?
        } else if stdin().is_terminal() || printer.is_json() {
            self.message
                .join(" ")
                .replace('\r', "")
                .replace('\n', "\r\n")
                .into_bytes()
        } else {
            stdin()
                .lock()
                .lines()
                .map_while(Result::ok)
                .collect::<Vec<String>>()
                .join("\r\n")
                .into_bytes()
        };

        let (from, to) = extract_envelope(&raw)?;
        let to_refs: Vec<&str> = to.iter().map(String::as_str).collect();
        client.send_message(raw, &from, &to_refs)?;
        printer.out(Message::new("Message successfully sent"))
    }
}
