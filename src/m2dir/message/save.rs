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
use io_m2dir::flag::types::M2dirFlags;
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::{
    m2dir::{arg::M2dirNameFlag, client::M2dirClient},
    shared::message::arg::MessageArg,
};

/// Save a message to an m2dir folder.
///
/// Appends a message to the specified m2dir. The message can be
/// passed as a positional file path, an inline raw string, or piped
/// via stdin (see [`MessageArg`] for resolution order). When flags
/// are passed, they are written to the `.meta/<id>.flags` file
/// alongside the message.
#[derive(Debug, Parser)]
pub struct M2dirMessageSaveCommand {
    #[command(flatten)]
    pub m2dir: M2dirNameFlag,

    /// Flag(s) to write to the new message's `.flags` metadata file.
    /// Each flag is an arbitrary UTF-8 string (e.g. `$seen`, `custom`).
    #[arg(long = "flag", short = 'f', num_args = 0..)]
    pub flags: Vec<String>,

    #[command(flatten)]
    pub message: MessageArg,
}

impl M2dirMessageSaveCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut M2dirClient) -> Result<()> {
        let store = client.open_store()?;
        let path = store.resolve_folder_path(&self.m2dir.inner)?;
        let m2dir = client.open_m2dir(path)?;

        let msg = self.message.parse()?;
        let entry = client.store(m2dir.clone(), msg.into_bytes())?;

        if !self.flags.is_empty() {
            let flags = M2dirFlags::from_iter(self.flags.iter().map(String::as_str));
            client.set_flags(&m2dir, entry.id(), flags)?;
        }

        printer.out(StoredMessage {
            id: entry.id().to_owned(),
            path: entry.path().as_str().to_owned(),
        })
    }
}

#[derive(Serialize)]
pub struct StoredMessage {
    id: String,
    path: String,
}

impl fmt::Display for StoredMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let id = &self.id;
        let path = &self.path;
        write!(f, "Message `{id}` successfully saved to {path}")
    }
}
