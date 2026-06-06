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

use anyhow::Result;
use clap::Parser;
use io_m2dir::flag::types::M2dirFlags;
use pimalaya_cli::printer::{Message, Printer};

use crate::m2dir::{
    arg::{M2dirNameFlag, MessageIdsArg},
    client::M2dirClient,
};

/// Set M2DIR flag(s) on message(s) (replaces any existing flags).
#[derive(Debug, Parser)]
pub struct M2dirFlagSetCommand {
    #[command(flatten)]
    pub ids: MessageIdsArg,

    #[command(flatten)]
    pub m2dir: M2dirNameFlag,

    /// Flag(s) to set on the message.
    #[arg(long = "flag", short = 'f', num_args = 1..)]
    pub flags: Vec<String>,
}

impl M2dirFlagSetCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut M2dirClient) -> Result<()> {
        let store = client.open_store()?;
        let path = store.resolve_folder_path(&self.m2dir.inner)?;
        let m2dir = client.open_m2dir(path)?;
        let flags = M2dirFlags::from_iter(self.flags.iter().map(String::as_str));

        for id in self.ids.inner {
            client.set_flags(&m2dir, &id, flags.clone())?;
        }

        printer.out(Message::new("M2dir flag(s) successfully changed"))
    }
}
