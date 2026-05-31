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
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::m2dir::{
    client::M2dirClient,
    message::{
        export::M2dirMessageExportCommand, get::M2dirMessageGetCommand,
        read::M2dirMessageReadCommand, save::M2dirMessageSaveCommand,
    },
};

/// Manage M2DIR messages.
///
/// A message is a complete email including headers and body. This
/// subcommand allows you to save, get, read and export messages.
#[derive(Debug, Subcommand)]
pub enum M2dirMessageCommand {
    Save(M2dirMessageSaveCommand),
    Get(M2dirMessageGetCommand),
    Read(M2dirMessageReadCommand),
    Export(M2dirMessageExportCommand),
}

impl M2dirMessageCommand {
    pub fn execute(self, printer: &mut impl Printer, client: M2dirClient) -> Result<()> {
        match self {
            Self::Save(cmd) => cmd.execute(printer, client),
            Self::Get(cmd) => cmd.execute(printer, client),
            Self::Read(cmd) => cmd.execute(printer, client),
            Self::Export(cmd) => cmd.execute(printer, client),
        }
    }
}
