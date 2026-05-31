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
    envelope::{get::M2dirEnvelopeGetCommand, list::M2dirEnvelopeListCommand},
};

/// Manage M2DIR envelopes.
///
/// An envelope contains header information about a message such as
/// date, subject, from, to, cc, bcc, etc.
#[derive(Debug, Subcommand)]
pub enum M2dirEnvelopeCommand {
    Get(M2dirEnvelopeGetCommand),
    List(M2dirEnvelopeListCommand),
}

impl M2dirEnvelopeCommand {
    pub fn execute(self, printer: &mut impl Printer, client: M2dirClient) -> Result<()> {
        match self {
            Self::Get(cmd) => cmd.execute(printer, client),
            Self::List(cmd) => cmd.execute(printer, client),
        }
    }
}
