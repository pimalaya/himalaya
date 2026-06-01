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

use crate::account::context::Account;
use crate::imap::{
    client::ImapClient,
    envelope::{
        get::ImapEnvelopeGetCommand, list::ImapEnvelopeListCommand,
        search::ImapEnvelopeSearchCommand, sort::ImapEnvelopeSortCommand,
        thread::ImapEnvelopeThreadCommand,
    },
};

/// Manage IMAP envelopes.
///
/// An envelope contains header information about a message such as
/// date, subject, from, to, cc, bcc, etc. This subcommand allows you
/// to get, list, search, sort, and thread envelopes.
#[derive(Debug, Subcommand)]
pub enum ImapEnvelopeCommand {
    Get(ImapEnvelopeGetCommand),
    List(ImapEnvelopeListCommand),
    Search(ImapEnvelopeSearchCommand),
    Sort(ImapEnvelopeSortCommand),
    Thread(ImapEnvelopeThreadCommand),
}

impl ImapEnvelopeCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut ImapClient,
    ) -> Result<()> {
        match self {
            Self::Get(cmd) => cmd.execute(printer, account, client),
            Self::List(cmd) => cmd.execute(printer, account, client),
            Self::Search(cmd) => cmd.execute(printer, account, client),
            Self::Sort(cmd) => cmd.execute(printer, account, client),
            Self::Thread(cmd) => cmd.execute(printer, client),
        }
    }
}
