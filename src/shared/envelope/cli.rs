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
use crate::shared::{
    client::EmailClient,
    envelope::{list::EnvelopeListCommand, search::EnvelopeSearchCommand},
};

/// Shared API to manage envelopes for the active account.
///
/// An envelope is a message headers subset. It is usually small, and
/// contains enough information to have an overall understanding of
/// what a message is about.
#[derive(Debug, Subcommand)]
pub enum EnvelopeCommand {
    #[command(visible_alias = "ls")]
    List(EnvelopeListCommand),
    #[command(visible_alias = "sr")]
    Search(EnvelopeSearchCommand),
}

impl EnvelopeCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, account, client),
            Self::Search(cmd) => cmd.execute(printer, account, client),
        }
    }
}
