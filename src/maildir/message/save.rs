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

use std::{fmt, path::PathBuf};

use anyhow::Result;
use clap::Parser;
use io_maildir::flag::MaildirFlags;
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::{
    maildir::{
        arg::{MaildirPathFlag, MaildirSubdirArg},
        client::MaildirClient,
        flag::arg::FlagArg,
    },
    shared::messages::arg::MessageArg,
};

/// Save a message to a mailbox.
///
/// Appends a message to the specified maildir. The message can be
/// passed as a positional file path, an inline raw string, or piped
/// via stdin (see [`MessageArg`] for resolution order).
#[derive(Debug, Parser)]
pub struct MaildirMessageSaveCommand {
    #[command(flatten)]
    pub maildir: MaildirPathFlag,

    /// The subdirectory of the Maildir
    #[arg(long, short, value_name = "DIR", value_enum)]
    #[arg(default_value = "new")]
    pub subdir: MaildirSubdirArg,

    /// The flags to add to the message.
    #[arg(long = "flag", short, num_args = 0..)]
    pub flags: Vec<FlagArg>,

    #[command(flatten)]
    pub message: MessageArg,
}

impl MaildirMessageSaveCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut MaildirClient) -> Result<()> {
        let maildir = client.resolve_maildir(&self.maildir.inner)?;
        let msg = self.message.parse()?;
        let flags = MaildirFlags::from_iter(self.flags.into_iter().map(Into::into));

        let (id, path) = client.store(maildir, self.subdir.into(), flags, msg.into_bytes())?;
        let path = PathBuf::from(path.into_string());

        printer.out(StoredMessage { id, path })
    }
}

#[derive(Serialize)]
pub struct StoredMessage {
    id: String,
    path: PathBuf,
}

impl fmt::Display for StoredMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let id = &self.id;
        let path = self.path.display();
        write!(f, "Message `{id}` successfully saved to {path}")
    }
}
