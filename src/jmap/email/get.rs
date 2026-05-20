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
    email::query::{EmailsChars, EmailsColors, EmailsTable},
};

/// Get JMAP emails by ID (Email/get).
///
/// Fetches and displays email envelopes as a table.
#[derive(Debug, Parser)]
pub struct JmapEmailGetCommand {
    /// The email ID(s) to retrieve.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,
}

impl JmapEmailGetCommand {
    pub fn execute(self, printer: &mut impl Printer, mut client: JmapClient) -> Result<()> {
        let output = client.email_get(self.ids.clone(), None, false, false, 0)?;

        for id in output.not_found {
            warn!("email `{id}` not found, ignoring it");
        }

        let table = EmailsTable {
            preset: client.account.table_preset().to_string(),
            arrangement: client.account.table_arrangement(),
            colors: EmailsColors {
                id: client.account.envelopes_list_table_id_color(),
                flags: client.account.envelopes_list_table_flags_color(),
                subject: client.account.envelopes_list_table_subject_color(),
                from: client.account.envelopes_list_table_from_color(),
                date: client.account.envelopes_list_table_date_color(),
            },
            chars: EmailsChars {
                unseen: client.account.envelopes_list_table_unseen_char(),
                flagged: client.account.envelopes_list_table_flagged_char(),
                attachment: client.account.envelopes_list_table_attachment_char(),
            },
            emails: output.emails,
        };

        printer.out(table)
    }
}
