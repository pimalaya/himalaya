use std::io::{Read, Write};

use anyhow::{bail, Result};
use clap::Parser;
use io_jmap::rfc8621::email_get::{JmapEmailGet, JmapEmailGetResult};
use log::warn;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::jmap::{account::JmapAccount, email::query::EmailsTable};

const READ_BUFFER_SIZE: usize = 16 * 1024;

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
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;

        let mut coroutine = JmapEmailGet::new(
            &jmap.session,
            &jmap.http_auth,
            self.ids.clone(),
            None,
            false,
            false,
            0,
        )?;
        let mut buf = [0u8; READ_BUFFER_SIZE];
        let mut arg: Option<&[u8]> = None;

        let (emails, not_found) = loop {
            match coroutine.resume(arg.take()) {
                JmapEmailGetResult::Ok {
                    emails, not_found, ..
                } => break (emails, not_found),
                JmapEmailGetResult::WantsRead => {
                    let n = jmap.stream.read(&mut buf)?;
                    arg = Some(&buf[..n]);
                }
                JmapEmailGetResult::WantsWrite(bytes) => {
                    jmap.stream.write_all(&bytes)?;
                    arg = None;
                }
                JmapEmailGetResult::Err(err) => bail!("{err}"),
            }
        };

        for id in not_found {
            warn!("email `{id}` not found, ignoring it");
        }

        let table = EmailsTable {
            preset: account.table_preset,
            arrangement: account.table_arrangement,
            emails,
        };

        printer.out(table)
    }
}
