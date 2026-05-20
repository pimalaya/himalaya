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
use log::warn;
use pimalaya_cli::printer::Printer;

use crate::jmap::{
    client::JmapClient,
    mailbox::query::{MailboxColors, MailboxesTable},
};

/// Get JMAP mailboxes by ID (Mailbox/get).
#[derive(Debug, Parser)]
pub struct JmapMailboxGetCommand {
    /// Mailbox ID(s) to retrieve.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,
}

impl JmapMailboxGetCommand {
    pub fn execute(self, printer: &mut impl Printer, mut client: JmapClient) -> Result<()> {
        let output = client.mailbox_get(Some(self.ids.clone()), None)?;

        for id in output.not_found {
            warn!("mailbox `{id}` not found, ignoring it");
        }

        let table = MailboxesTable {
            preset: client.account.table_preset().to_string(),
            colors: MailboxColors {
                id: client.account.mailboxes_list_table_id_color(),
                name: client.account.mailboxes_list_table_name_color(),
                total: client.account.mailboxes_list_table_total_color(),
                unread: client.account.mailboxes_list_table_unread_color(),
            },
            mailboxes: output.mailboxes,
        };

        printer.out(table)
    }
}
