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
    client::M2dirClient, create::M2dirMailboxCreateCommand, delete::M2dirMailboxDeleteCommand,
    list::M2dirMailboxListCommand,
};

/// m2dir CLI.
///
/// Protocol-specific entry point for the m2dir backend. Wider
/// operations (envelopes, flags, messages) go through the shared
/// commands `himalaya envelopes`, `himalaya flags`, `himalaya
/// messages` with `--backend m2dir`.
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum M2dirCommand {
    Create(M2dirMailboxCreateCommand),
    Delete(M2dirMailboxDeleteCommand),
    List(M2dirMailboxListCommand),
}

impl M2dirCommand {
    pub fn execute(self, printer: &mut impl Printer, client: M2dirClient) -> Result<()> {
        match self {
            Self::Create(cmd) => cmd.execute(printer, client),
            Self::Delete(cmd) => cmd.execute(printer, client),
            Self::List(cmd) => cmd.execute(printer, client),
        }
    }
}
