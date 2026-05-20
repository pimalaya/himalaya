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
    fmt,
    io::{BufRead, IsTerminal, stdin},
    path::PathBuf,
};

use anyhow::Result;
use clap::Parser;
use io_maildir::{flag::Flags, maildir::Maildir};
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::maildir::{
    arg::{MaildirPathFlag, MaildirSubdirArg},
    client::MaildirClient,
    flag::arg::FlagArg,
};

/// Save a message to a mailbox.
///
/// This command appends a message to the specified mailbox. The
/// message is read from stdin in RFC 5322 format (raw email).
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

    /// The raw message, including headers and body.
    #[arg(trailing_var_arg = true)]
    #[arg(name = "message", value_name = "MESSAGE")]
    pub message: Vec<String>,
}

impl MaildirMessageSaveCommand {
    pub fn execute(self, printer: &mut impl Printer, client: MaildirClient) -> Result<()> {
        let maildir = match Maildir::try_from(self.maildir.inner.clone()) {
            Ok(maildir) => maildir,
            Err(_) => Maildir::try_from(client.root.join(&self.maildir.inner))?,
        };

        let msg = if stdin().is_terminal() || printer.is_json() {
            self.message
                .join(" ")
                .replace('\r', "")
                .replace('\n', "\r\n")
        } else {
            stdin()
                .lock()
                .lines()
                .map_while(Result::ok)
                .collect::<Vec<String>>()
                .join("\r\n")
        };

        let flags = Flags::from_iter(self.flags.into_iter().map(Into::into));

        let (id, path) = client.store(maildir, self.subdir.into(), flags, msg.into_bytes())?;

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
