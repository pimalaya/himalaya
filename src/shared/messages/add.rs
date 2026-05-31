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

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_email::flag::Flag;
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::shared::{client::EmailClient, flags::arg::FlagArg, messages::arg::MessageArg};

/// Add a raw RFC 5322 message to a mailbox.
///
/// The message can be passed as a positional file path, an inline raw
/// string, or piped via stdin (see [`MessageArg`] for resolution
/// order). IMAP appends via `APPEND` (RFC 3501); JMAP uploads the
/// blob and imports it via `Email/import` (the destination mailbox
/// is resolved from `--mailbox` by exact-match name); Maildir writes
/// a new file under the target maildir's `cur/` subdir using the
/// standard tmp-then-rename delivery protocol.
#[derive(Debug, Parser)]
pub struct MessageAddCommand {
    /// Destination mailbox name or path. Mandatory.
    #[arg(long = "mailbox", short = 'm', value_name = "NAME")]
    pub mailbox: String,

    /// Flag(s) to set on the new message. Optional.
    #[arg(long = "flag", short = 'f', value_name = "FLAG", num_args = 0..)]
    pub flag: Vec<FlagArg>,

    #[command(flatten)]
    pub message: MessageArg,
}

impl MessageAddCommand {
    pub fn execute(self, printer: &mut impl Printer, mut client: EmailClient) -> Result<()> {
        let raw = self.message.parse()?.into_bytes();
        let flags: Vec<Flag> = self.flag.iter().map(Into::into).collect();
        let id = client.add_message(&self.mailbox, &flags, raw)?;
        printer.out(MessageAddOutput { id })
    }
}

#[derive(Serialize)]
struct MessageAddOutput {
    id: String,
}

impl fmt::Display for MessageAddOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Message {} successfully added", self.id)
    }
}
