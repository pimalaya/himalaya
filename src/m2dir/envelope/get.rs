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

use anyhow::{Result, bail};
use clap::Parser;
use comfy_table::{Cell, Row, Table};
use mail_parser::{Header, MessageParser};
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::account::context::Account;
use crate::m2dir::{
    arg::{M2dirNameFlag, MessageIdArg},
    client::M2dirClient,
};

/// Get a single M2DIR envelope.
///
/// Resolves the message identified by `ID` inside the given m2dir
/// folder, parses its headers, and prints them.
#[derive(Debug, Parser)]
pub struct M2dirEnvelopeGetCommand {
    #[command(flatten)]
    pub m2dir: M2dirNameFlag,
    #[command(flatten)]
    pub id: MessageIdArg,
}

impl M2dirEnvelopeGetCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut M2dirClient,
    ) -> Result<()> {
        let store = client.open_store()?;
        let path = store.resolve_folder_path(&self.m2dir.inner)?;
        let m2dir = client.open_m2dir(path)?;
        let (entry, bytes) = client.get(m2dir, &self.id.inner)?;

        let Some(parsed) = MessageParser::new().parse_headers(&bytes) else {
            let path = entry.path();
            bail!("Invalid MIME message at {path}");
        };

        let table = EnvelopeTable {
            preset: account.table_preset().to_string(),
            headers: parsed.headers(),
        };

        printer.out(table)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct EnvelopeTable<'a> {
    #[serde(skip)]
    pub preset: String,
    pub headers: &'a [Header<'a>],
}

impl fmt::Display for EnvelopeTable<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(&self.preset)
            .set_header(Row::from([Cell::new("HEADER"), Cell::new("VALUE")]));

        for header in self.headers {
            writeln!(f, "{}: {:?}", header.name.as_str(), header.value)?;
        }

        Ok(())
    }
}
