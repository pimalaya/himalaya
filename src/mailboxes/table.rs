use std::fmt;

use comfy_table::{Cell, ContentArrangement, Row, Table};
use io_email::mailbox::Mailbox;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct MailboxesTable {
    #[serde(skip)]
    pub preset: String,
    #[serde(skip)]
    pub arrangement: ContentArrangement,
    pub mailboxes: Vec<Mailbox>,
}

impl fmt::Display for MailboxesTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(&self.preset)
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from([
                Cell::new("ID"),
                Cell::new("NAME"),
                Cell::new("ROLE"),
                Cell::new("ATTRIBUTES"),
            ]))
            .add_rows(self.mailboxes.iter().map(|m| {
                let mut row = Row::new();
                row.max_height(1);
                row.add_cell(Cell::new(&m.id));
                row.add_cell(Cell::new(&m.name));
                row.add_cell(match &m.role {
                    Some(role) => Cell::new(format!("{role:?}")),
                    None => Cell::new(""),
                });
                row.add_cell(Cell::new(m.attributes.join(", ")));
                row
            }));

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}
