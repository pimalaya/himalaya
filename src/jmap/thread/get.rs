use std::{
    fmt,
    io::{Read, Write},
};

use anyhow::{bail, Result};
use clap::Parser;
use comfy_table::{Cell, Row, Table};
use io_jmap::rfc8621::{
    thread::Thread,
    thread_get::{JmapThreadGet, JmapThreadGetResult},
};
use log::warn;
use pimalaya_toolbox::terminal::printer::Printer;
use serde::Serialize;

use crate::jmap::account::JmapAccount;

const READ_BUFFER_SIZE: usize = 16 * 1024;

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
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;

        let mut coroutine = JmapThreadGet::new(&jmap.session, &jmap.http_auth, self.ids.clone())?;
        let mut buf = [0u8; READ_BUFFER_SIZE];
        let mut arg: Option<&[u8]> = None;

        let (threads, not_found) = loop {
            match coroutine.resume(arg.take()) {
                JmapThreadGetResult::Ok {
                    threads, not_found, ..
                } => break (threads, not_found),
                JmapThreadGetResult::WantsRead => {
                    let n = jmap.stream.read(&mut buf)?;
                    arg = Some(&buf[..n]);
                }
                JmapThreadGetResult::WantsWrite(bytes) => {
                    jmap.stream.write_all(&bytes)?;
                    arg = None;
                }
                JmapThreadGetResult::Err(err) => bail!("{err}"),
            }
        };

        for id in not_found {
            warn!("thread `{id}` not found, ignoring it");
        }

        printer.out(ThreadsTable {
            preset: account.table_preset,
            threads,
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
