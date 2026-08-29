//! # Mailbox list
//!
//! The `mailbox list` command, tabling the mailboxes of the active
//! account.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use comfy_table::{Cell, Color, ContentArrangement, Row, Table};
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    email::mailbox::Mailbox,
    shared::{client::EmailClient, table::style_from_preset},
};

/// List the mailboxes of the active account.
#[derive(Debug, Parser)]
pub struct MailboxListCommand {
    /// Fill the TOTAL and UNREAD columns.
    ///
    /// JMAP returns the counts in the same response, but IMAP issues one
    /// extra `STATUS` per mailbox, which is slow on an account with many.
    /// Maildir does not implement counts at all.
    #[arg(long)]
    pub counts: bool,
    /// Maximum width of the rendered table, in terminal columns.
    ///
    /// Overrides the auto-detected width, columns shrinking with an
    /// ellipsis as needed, which is what piping through `less -S` wants.
    #[arg(long = "max-width", short = 'w')]
    #[arg(value_name = "COLUMNS")]
    pub max_width: Option<u16>,
}

impl MailboxListCommand {
    /// Lists the mailboxes and prints them as a table.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        let mailboxes = client.list_mailboxes(self.counts)?;

        let mailboxes = Mailboxes {
            preset: account.table_preset().to_string(),
            arrangement: account.table_arrangement(),
            max_width: self.max_width,
            with_counts: self.counts,
            colors: MailboxColors {
                id: account.mailboxes_list_table_id_color(),
                name: account.mailboxes_list_table_name_color(),
                total: account.mailboxes_list_table_total_color(),
                unread: account.mailboxes_list_table_unread_color(),
            },
            mailboxes,
        };

        printer.out(mailboxes)
    }
}

/// Per-column colors of the mailboxes table.
#[derive(Clone, Copy, Debug)]
struct MailboxColors {
    id: Color,
    name: Color,
    total: Color,
    unread: Color,
}

/// The `mailbox list` output, a table of mailboxes.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct Mailboxes {
    /// The `comfy_table` preset string the table renders with.
    #[serde(skip)]
    pub preset: String,
    /// The column arrangement the table renders with.
    #[serde(skip)]
    pub arrangement: ContentArrangement,
    /// The width the table is capped at, when one was asked for.
    #[serde(skip)]
    pub max_width: Option<u16>,
    /// Whether the count columns are drawn.
    #[serde(skip)]
    pub with_counts: bool,
    #[serde(skip)]
    colors: MailboxColors,
    /// The mailboxes, in the order the backend returned them.
    pub mailboxes: Vec<Mailbox>,
}

impl fmt::Display for Mailboxes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        let mut header = vec![Cell::new("ID"), Cell::new("NAME")];
        if self.with_counts {
            header.push(Cell::new("TOTAL"));
            header.push(Cell::new("UNREAD"));
        }

        table
            .load_style(style_from_preset(&self.preset))
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from(header))
            .add_rows(self.mailboxes.iter().map(|m| {
                let mut row = Row::new();
                row.max_height(1);
                row.add_cell(Cell::new(&m.id).fg(self.colors.id));
                row.add_cell(Cell::new(&m.name).fg(self.colors.name));
                if self.with_counts {
                    row.add_cell(count_cell(m.total).fg(self.colors.total));
                    row.add_cell(count_cell(m.unread).fg(self.colors.unread));
                }
                row
            }));

        if let Some(width) = self.max_width {
            table.set_width(width);
        }

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}

/// Renders a count, or an empty cell when the backend gave none.
fn count_cell(value: Option<u64>) -> Cell {
    match value {
        Some(n) => Cell::new(n),
        None => Cell::new(""),
    }
}
