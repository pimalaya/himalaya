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

use std::fmt;

use anyhow::Result;
use clap::Parser;
use comfy_table::{Cell, Row, Table};
use io_jmap::rfc8621::thread::Thread;
use log::warn;
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::account::context::Account;
use crate::jmap::client::JmapClient;

/// Get JMAP threads by ID (Thread/get).
///
/// Each thread contains an ordered list of email IDs in the thread.
#[derive(Debug, Parser)]
pub struct JmapThreadGetCommand {
    /// Thread ID(s) to retrieve.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,
}

impl JmapThreadGetCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut JmapClient,
    ) -> Result<()> {
        let output = client.thread_get(self.ids.clone())?;

        for id in output.not_found {
            warn!("thread `{id}` not found, ignoring it");
        }

        printer.out(ThreadsTable {
            preset: account.table_preset().to_string(),
            threads: output.threads,
        })
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ThreadsTable {
    #[serde(skip)]
    pub preset: String,
    pub threads: Vec<Thread>,
}

impl fmt::Display for ThreadsTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(&self.preset)
            .set_header(Row::from([Cell::new("ID"), Cell::new("EMAIL IDS")]))
            .add_rows(
                self.threads
                    .iter()
                    .map(|t| Row::from([Cell::new(&t.id), Cell::new(t.email_ids.join(", "))])),
            );

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}
