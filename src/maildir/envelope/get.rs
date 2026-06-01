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
use mail_parser::Header;
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::account::context::Account;
use crate::maildir::{
    arg::{MaildirPathFlag, MessageIdArg},
    client::MaildirClient,
};

/// Get a single MAILDIR envelope.
///
/// This command displays detailed envelope information for a specific
/// message, including all header fields like date, subject, from, to,
/// cc, bcc, reply-to, message-id, and in-reply-to.
#[derive(Debug, Parser)]
pub struct MaildirEnvelopeGetCommand {
    #[command(flatten)]
    pub maildir: MaildirPathFlag,
    #[command(flatten)]
    pub id: MessageIdArg,
}

impl MaildirEnvelopeGetCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut MaildirClient,
    ) -> Result<()> {
        let maildir = client.resolve_maildir(&self.maildir.inner)?;

        let message = client.get(maildir, &self.id.inner)?;

        let path = message.path().clone();

        let Some(parsed) = message.headers() else {
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
