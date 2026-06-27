use std::fmt;

use anyhow::{Result, bail};
use clap::Parser;
use comfy_table::{Cell, Row, Table};
use io_jmap::rfc8621::vacation_response::{JmapVacationResponse, VACATION_RESPONSE_CAPABILITY};
use pimalaya_cli::printer::{Message, Printer};
use serde::Serialize;

use crate::account::context::Account;
use crate::jmap::client::JmapClient;

/// Get the JMAP vacation response (VacationResponse/get).
#[derive(Debug, Parser)]
pub struct JmapVacationGetCommand;

impl JmapVacationGetCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut JmapClient,
    ) -> Result<()> {
        let has_vacation = client
            .session()
            .map(|s| s.capabilities.contains_key(VACATION_RESPONSE_CAPABILITY))
            .unwrap_or(false);

        if !has_vacation {
            bail!("Vacation response is not supported by the server");
        }

        let Some(vacation) = client.vacation_response_get()? else {
            return printer.out(Message::new("No vacation response configured"));
        };

        let table = VacationTable {
            preset: account.table_preset().to_string(),
            vacation,
        };

        printer.out(table)
    }
}

/// Renderable table of the vacation response settings.
#[derive(Clone, Debug, Serialize)]
#[serde(transparent)]
pub struct VacationTable {
    #[serde(skip)]
    pub preset: String,
    pub vacation: JmapVacationResponse,
}

impl fmt::Display for VacationTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();
        let v = &self.vacation;

        table
            .load_preset(&self.preset)
            .set_header(Row::from([Cell::new("KEY"), Cell::new("VALUE")]));

        table.add_row(Row::from([
            Cell::new("Enabled"),
            Cell::new(if v.is_enabled { "true" } else { "" }),
        ]));

        if let Some(d) = &v.from_date {
            table.add_row(Row::from([Cell::new("From"), Cell::new(d)]));
        }

        if let Some(d) = &v.to_date {
            table.add_row(Row::from([Cell::new("To"), Cell::new(d)]));
        }

        if let Some(s) = &v.subject {
            table.add_row(Row::from([Cell::new("Subject"), Cell::new(s)]));
        }

        if let Some(b) = &v.text_body {
            table.add_row(Row::from([Cell::new("Body (plain)"), Cell::new(b)]));
        }

        if let Some(b) = &v.html_body {
            table.add_row(Row::from([Cell::new("Body (HTML)"), Cell::new(b)]));
        }

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}
