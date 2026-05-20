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
    io::{stdin, IsTerminal, Read},
    path::PathBuf,
};

use anyhow::{bail, Result};
use clap::Parser;
use io_email::flag::Flag;
use pimalaya_cli::printer::Message;
use pimalaya_cli::printer::Printer;

use crate::shared::{client::EmailClient, flags::arg::FlagArg};

/// Add a raw RFC 5322 message to a mailbox.
///
/// The message body is read from stdin by default; pass `--file
/// <PATH>` to read from a file instead. IMAP appends via `APPEND`
/// (RFC 3501); JMAP uploads the blob and imports it via `Email/import`
/// (the destination mailbox is resolved from `--mailbox` by exact-match
/// name); Maildir writes a new file under the target maildir's `cur/`
/// subdir using the standard tmp-then-rename delivery protocol.
#[derive(Debug, Parser)]
pub struct MessageAddCommand {
    /// Destination mailbox name or path. Mandatory.
    #[arg(long = "mailbox", short = 'm', value_name = "NAME")]
    pub mailbox: String,

    /// Flag(s) to set on the new message. Optional.
    #[arg(long = "flag", short = 'f', value_name = "FLAG", num_args = 0..)]
    pub flag: Vec<FlagArg>,

    /// Read the raw message from this file instead of stdin.
    #[arg(long = "file", value_name = "PATH")]
    pub file: Option<PathBuf>,
}

impl MessageAddCommand {
    pub fn execute(self, printer: &mut impl Printer, mut client: EmailClient) -> Result<()> {
        let raw = read_raw(&self.file)?;
        let flags: Vec<Flag> = self.flag.iter().map(Into::into).collect();
        client.add_message(&self.mailbox, &flags, raw)?;
        printer.out(Message::new("Message successfully added"))
    }
}

fn read_raw(file: &Option<PathBuf>) -> Result<Vec<u8>> {
    if let Some(path) = file {
        return Ok(std::fs::read(path)?);
    }

    if stdin().is_terminal() {
        bail!(
            "`messages add` reads the raw message from stdin or `--file <PATH>` — \
             nothing was provided"
        );
    }

    let mut buf = Vec::new();
    stdin().read_to_end(&mut buf)?;
    Ok(buf)
}
