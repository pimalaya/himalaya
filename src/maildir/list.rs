use std::{fmt, path::PathBuf};

use anyhow::{bail, Result};
use clap::Parser;
use comfy_table::{Cell, Row, Table};
use io_fs::runtimes::std::handle;
use io_maildir::{coroutines::list_maildirs::*, maildir::Maildir};
use pimalaya_toolbox::terminal::printer::Printer;
use serde::Serialize;

use crate::maildir::account::MaildirAccount;

/// List, search and filter maildirs.
///
/// This command allows you to list maildirs from your MAILDIR account.
/// By default, only subscribed maildirs are listed. Use --all to
/// list all maildirs.
#[derive(Debug, Parser)]
pub struct MaildirMailboxListCommand;

impl MaildirMailboxListCommand {
    pub fn execute(self, printer: &mut impl Printer, account: MaildirAccount) -> Result<()> {
        let mut arg = None;
        let mut coroutine = ListMaildirs::new(account.backend.root);

        let maildirs = loop {
            match coroutine.resume(arg.take()) {
                ListMaildirsResult::Io(io) => arg = Some(handle(io)?),
                ListMaildirsResult::Ok(maildirs) => break maildirs,
                ListMaildirsResult::Err(err) => bail!(err),
            }
        };

        let table = MaildirsTable {
            preset: account.table_preset,
            rows: maildirs.into_iter().map(From::from).collect(),
        };

        printer.out(table)
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct MaildirsTable {
    #[serde(skip)]
    pub preset: String,
    #[serde(rename = "maildirs")]
    pub rows: Vec<MaildirRow>,
}

impl fmt::Display for MaildirsTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(&self.preset)
            .set_header(Row::from([Cell::new("NAME"), Cell::new("PATH")]))
            .add_rows(self.rows.iter().map(|m| {
                let mut row = Row::new();

                row.max_height(1)
                    .add_cell(Cell::new(&m.name))
                    .add_cell(Cell::new(format!("{}", m.path.display())));

                row
            }));

        writeln!(f)?;
        write!(f, "{table}")?;
        writeln!(f)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct MaildirRow {
    pub name: String,
    pub path: PathBuf,
}

impl From<Maildir> for MaildirRow {
    fn from(maildir: Maildir) -> Self {
        Self {
            name: maildir.name().unwrap_or("Unknown").to_owned(),
            path: maildir.as_ref().to_owned(),
        }
    }
}
