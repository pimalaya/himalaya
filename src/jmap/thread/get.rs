use std::fmt;

use anyhow::Result;
use clap::Parser;
use comfy_table::{Cell, Row, Table};
use io_jmap::rfc8621::thread::JmapThread;
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

/// Renderable table of threads and their email IDs.
#[derive(Clone, Debug, Serialize)]
pub struct ThreadsTable {
    #[serde(skip)]
    pub preset: String,
    pub threads: Vec<JmapThread>,
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
