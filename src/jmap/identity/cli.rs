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
use crate::jmap::{
    client::JmapClient,
    identity::{
        create::JmapIdentityCreateCommand, delete::JmapIdentityDeleteCommand,
        get::JmapIdentityGetCommand, update::JmapIdentityUpdateCommand,
    },
};

/// Manage JMAP sender identities.
#[derive(Debug, Subcommand)]
pub enum JmapIdentityCommand {
    /// Fetch identities (Identity/get).
    #[command(aliases = ["lst", "list"])]
    Get(JmapIdentityGetCommand),
    /// Create a new identity (Identity/set).
    #[command(aliases = ["add", "new"])]
    Create(JmapIdentityCreateCommand),
    /// Update an existing identity (Identity/set).
    #[command(alias = "edit")]
    Update(JmapIdentityUpdateCommand),
    /// Delete an identity (Identity/set).
    #[command(aliases = ["remove", "rm"])]
    Delete(JmapIdentityDeleteCommand),
}

impl JmapIdentityCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut JmapClient,
    ) -> Result<()> {
        match self {
            Self::Get(cmd) => cmd.execute(printer, account, client),
            Self::Create(cmd) => cmd.execute(printer, client),
            Self::Update(cmd) => cmd.execute(printer, client),
            Self::Delete(cmd) => cmd.execute(printer, client),
        }
    }
}
