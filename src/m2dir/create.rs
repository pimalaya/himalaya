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
use pimalaya_cli::printer::{Message, Printer};

use crate::m2dir::{arg::M2dirNameArg, client::M2dirClient};

/// Create the given m2dir folder.
///
/// Initialises the m2store at the client root if needed, then creates
/// the m2dir folder named after `name` (relative to the store root).
#[derive(Debug, Parser)]
pub struct M2dirMailboxCreateCommand {
    #[command(flatten)]
    pub m2dir_name: M2dirNameArg,
}

impl M2dirMailboxCreateCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut M2dirClient) -> Result<()> {
        client.init_store()?;
        client.create_mailbox(&self.m2dir_name.inner)?;
        printer.out(Message::new("m2dir folder successfully created"))
    }
}
