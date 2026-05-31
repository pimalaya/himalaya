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
    envelope::cli::M2dirEnvelopeCommand, flag::cli::M2dirFlagCommand,
    list::M2dirMailboxListCommand, message::cli::M2dirMessageCommand,
};

/// m2dir CLI.
///
/// Protocol-specific entry point for the m2dir backend. Mailbox and
/// per-folder operations (messages, flags, envelopes) live here;
/// cross-backend shared commands also dispatch here when
/// `--backend m2dir` is passed.
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum M2dirCommand {
    Create(M2dirMailboxCreateCommand),
    Delete(M2dirMailboxDeleteCommand),
    List(M2dirMailboxListCommand),

    #[command(subcommand)]
    #[command(aliases = ["msgs", "msg"])]
    Messages(M2dirMessageCommand),
    #[command(subcommand)]
    Flags(M2dirFlagCommand),
    #[command(subcommand)]
    Envelopes(M2dirEnvelopeCommand),
}

impl M2dirCommand {
    pub fn execute(self, printer: &mut impl Printer, client: M2dirClient) -> Result<()> {
        match self {
            Self::Create(cmd) => cmd.execute(printer, client),
            Self::Delete(cmd) => cmd.execute(printer, client),
            Self::List(cmd) => cmd.execute(printer, client),

            Self::Messages(cmd) => cmd.execute(printer, client),
            Self::Flags(cmd) => cmd.execute(printer, client),
            Self::Envelopes(cmd) => cmd.execute(printer, client),
        }
    }
}
