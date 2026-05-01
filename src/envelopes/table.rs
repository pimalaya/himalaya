use std::fmt;

use comfy_table::{Cell, ContentArrangement, Row, Table};
use io_email::envelope::Envelope;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct EnvelopesTable {
    #[serde(skip)]
    pub preset: String,
    #[serde(skip)]
    pub arrangement: ContentArrangement,
    pub envelopes: Vec<Envelope>,
}

impl fmt::Display for EnvelopesTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(&self.preset)
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from([
                Cell::new("ID"),
                Cell::new("FLAGS"),
                Cell::new("SUBJECT"),
                Cell::new("FROM"),
                Cell::new("DATE"),
            ]))
            .add_rows(self.envelopes.iter().map(|e| {
                let mut row = Row::new();
                row.max_height(1);
                row.add_cell(Cell::new(&e.id));
                row.add_cell(Cell::new(
                    e.flags
                        .iter()
                        .map(|f| format!("{f:?}"))
                        .collect::<Vec<_>>()
                        .join(", "),
                ));
                row.add_cell(Cell::new(&e.subject));
                row.add_cell(Cell::new(
                    e.from
                        .iter()
                        .map(|a| match &a.name {
                            Some(name) if !name.is_empty() => name.clone(),
                            _ => a.email.clone(),
                        })
                        .collect::<Vec<_>>()
                        .join(", "),
                ));
                row.add_cell(Cell::new(&e.date));
                row
            }));

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}
