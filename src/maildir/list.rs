use std::{fmt, path::PathBuf};

use anyhow::Result;
use clap::Parser;
use comfy_table::{Cell, Color, Row, Table};
use io_maildir::maildir::types::Maildir;
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::account::context::Account;
use crate::maildir::client::MaildirClient;

/// List Maildir folders.
///
/// Scans the account root and lists every folder found, with its name
/// and filesystem path.
#[derive(Debug, Parser)]
pub struct MaildirMailboxListCommand;

impl MaildirMailboxListCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut MaildirClient,
    ) -> Result<()> {
        let maildirs = client.list_maildirs()?;

        let table = MaildirsTable {
            preset: account.table_preset().to_string(),
            name_color: account.mailboxes_list_table_name_color(),
            rows: maildirs.into_iter().map(From::from).collect(),
        };

        printer.out(table)
    }
}

/// Renderable table of Maildir folders.
#[derive(Clone, Debug, Serialize)]
pub struct MaildirsTable {
    #[serde(skip)]
    pub preset: String,
    #[serde(skip)]
    pub name_color: Color,
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
                    .add_cell(Cell::new(&m.name).fg(self.name_color))
                    .add_cell(Cell::new(format!("{}", m.path.display())));

                row
            }));

        writeln!(f)?;
        write!(f, "{table}")?;
        writeln!(f)?;
        Ok(())
    }
}

/// One row of the Maildir folders table: name and path.
#[derive(Clone, Debug, Serialize)]
pub struct MaildirRow {
    pub name: String,
    pub path: PathBuf,
}

impl From<Maildir> for MaildirRow {
    fn from(maildir: Maildir) -> Self {
        Self {
            name: maildir.name().unwrap_or("Unknown").to_owned(),
            path: PathBuf::from(maildir.path().as_str()),
        }
    }
}
