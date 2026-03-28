use std::fmt;

use anyhow::{bail, Result};
use clap::Parser;
use comfy_table::{Cell, Row, Table};
use io_jmap::{
    rfc8620::types::session::capabilities::VACATION_RESPONSE,
    rfc8621::coroutines::vacation_response_get::{
        JmapVacationResponseGet, JmapVacationResponseGetResult,
    },
    rfc8621::types::vacation_response::VacationResponse,
};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};
use serde::Serialize;

use crate::jmap::account::JmapAccount;

/// Get the JMAP vacation response (VacationResponse/get).
#[derive(Debug, Parser)]
pub struct GetVacationCommand;

impl GetVacationCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;

        // Skip the request if the server does not advertise the
        // vacation-response capability.
        let has_vacation = jmap.session.capabilities.contains_key(VACATION_RESPONSE);

        if !has_vacation {
            bail!("Vacation response is not supported by the server");
        }

        let mut coroutine = JmapVacationResponseGet::new(&jmap.session, &jmap.http_auth)?;
        let mut arg = None;

        let vacation = loop {
            match coroutine.resume(arg.take()) {
                JmapVacationResponseGetResult::Io { io } => {
                    arg = Some(handle(&mut jmap.stream, io)?)
                }
                JmapVacationResponseGetResult::Ok {
                    vacation_response, ..
                } => break vacation_response,
                JmapVacationResponseGetResult::Err { err, .. } => bail!(err),
            }
        };

        let Some(vacation) = vacation else {
            return printer.out(Message::new("No vacation response configured"));
        };

        let table = VacationTable {
            preset: account.table_preset,
            vacation,
        };

        printer.out(table)
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(transparent)]
pub struct VacationTable {
    #[serde(skip)]
    pub preset: String,
    pub vacation: VacationResponse,
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
