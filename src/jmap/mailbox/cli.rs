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
    mailbox::{
        create::JmapMailboxCreateCommand, destroy::JmapMailboxDestroyCommand,
        get::JmapMailboxGetCommand, query::JmapMailboxQueryCommand,
        update::JmapMailboxUpdateCommand,
    },
};

/// Manage JMAP mailboxes.
#[derive(Debug, Subcommand)]
pub enum JmapMailboxCommand {
    Get(JmapMailboxGetCommand),
    Query(JmapMailboxQueryCommand),
    #[command(visible_aliases = ["add", "new"])]
    Create(JmapMailboxCreateCommand),
    Update(JmapMailboxUpdateCommand),
    #[command(visible_aliases = ["delete", "del", "remove", "rm"])]
    Destroy(JmapMailboxDestroyCommand),
}

impl JmapMailboxCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut JmapClient,
    ) -> Result<()> {
        match self {
            Self::Get(cmd) => cmd.execute(printer, account, client),
            Self::Query(cmd) => cmd.execute(printer, account, client),
            Self::Create(cmd) => cmd.execute(printer, client),
            Self::Update(cmd) => cmd.execute(printer, client),
            Self::Destroy(cmd) => cmd.execute(printer, client),
        }
    }
}
