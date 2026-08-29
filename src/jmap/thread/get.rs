//! # JMAP thread get
//!
//! The `jmap thread get` command, RFC 8621 `Thread/get`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use comfy_table::{Cell, Row, Table};
use io_jmap::rfc8621::thread::JmapThread;
use log::warn;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account, jmap::client::JmapClient, shared::table::style_from_preset,
};

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
    /// Fetches the threads and tables the emails they hold.
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

/// The threads rendered as a table, each with the emails it holds.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct ThreadsTable {
    /// The `comfy_table` preset string the table renders with.
    #[serde(skip)]
    pub preset: String,
    /// The threads, in the order the server returned them.
    pub threads: Vec<JmapThread>,
}

impl fmt::Display for ThreadsTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_style(style_from_preset(&self.preset))
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
