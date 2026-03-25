use anyhow::{bail, Result};
use clap::Parser;
use io_jmap::coroutines::email_get::{GetJmapEmails, GetJmapEmailsResult};
use io_stream::runtimes::std::handle;
use log::warn;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::jmap::{account::JmapAccount, email::query::EmailsTable};

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

        let mut coroutine =
            GetJmapEmails::new(jmap.context, self.ids.clone(), None, false, false, 0)?;
        let mut arg = None;

        let (emails, not_found) = loop {
            match coroutine.resume(arg.take()) {
                GetJmapEmailsResult::Io(io) => arg = Some(handle(&mut jmap.stream, io)?),
                GetJmapEmailsResult::Ok {
                    emails, not_found, ..
                } => break (emails, not_found),
                GetJmapEmailsResult::Err { err, .. } => bail!(err),
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
