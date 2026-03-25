use std::fmt;

use anyhow::{bail, Result};
use clap::Parser;
use comfy_table::{Cell, Row, Table};
use io_jmap::{
    coroutines::identity_get::{GetJmapIdentities, GetJmapIdentitiesResult},
    types::identity::Identity,
};
use io_stream::runtimes::std::handle;
use log::warn;
use pimalaya_toolbox::terminal::printer::Printer;
use serde::Serialize;

use crate::jmap::account::JmapAccount;

/// Get JMAP identities (Identity/get).
///
/// Lists sender identities available for sending email. Pass no IDs to
/// list all identities.
#[derive(Debug, Parser)]
pub struct GetIdentityCommand {
    /// Identity ID(s) to retrieve (omit to get all).
    #[arg(value_name = "ID")]
    pub ids: Option<Vec<String>>,
}

impl GetIdentityCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;

        let mut coroutine = GetJmapIdentities::new(jmap.context, self.ids)?;
        let mut arg = None;

        let (identities, not_found) = loop {
            match coroutine.resume(arg.take()) {
                GetJmapIdentitiesResult::Io(io) => arg = Some(handle(&mut jmap.stream, io)?),
                GetJmapIdentitiesResult::Ok {
                    identities,
                    not_found,
                    ..
                } => break (identities, not_found),
                GetJmapIdentitiesResult::Err { err, .. } => bail!(err),
            }
        };

        for id in not_found {
            warn!("identity `{id}` not found");
        }

        let table = IdentitiesTable {
            preset: account.table_preset,
            identities,
        };

        printer.out(table)
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(transparent)]
pub struct IdentitiesTable {
    #[serde(skip)]
    pub preset: String,
    pub identities: Vec<Identity>,
}

impl fmt::Display for IdentitiesTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(&self.preset)
            .set_header(Row::from([
                Cell::new("ID"),
                Cell::new("NAME"),
                Cell::new("EMAIL"),
            ]))
            .add_rows(
                self.identities.iter().map(|i| {
                    Row::from([Cell::new(&i.id), Cell::new(&i.name), Cell::new(&i.email)])
                }),
            );

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}
