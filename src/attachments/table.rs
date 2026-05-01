use std::fmt;

use comfy_table::{Cell, ContentArrangement, Row, Table};
use serde::Serialize;

/// One row of the `attachments list` output.
#[derive(Clone, Debug, Serialize)]
pub struct AttachmentEntry {
    pub index: usize,
    pub filename: String,
    pub mime: String,
    pub size: usize,
    pub inline: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct AttachmentsTable {
    #[serde(skip)]
    pub preset: String,
    #[serde(skip)]
    pub arrangement: ContentArrangement,
    pub attachments: Vec<AttachmentEntry>,
}

impl fmt::Display for AttachmentsTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(&self.preset)
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from([
                Cell::new("INDEX"),
                Cell::new("FILENAME"),
                Cell::new("MIME"),
                Cell::new("SIZE"),
                Cell::new("INLINE"),
            ]))
            .add_rows(self.attachments.iter().map(|a| {
                let mut row = Row::new();
                row.max_height(1);
                row.add_cell(Cell::new(a.index));
                row.add_cell(Cell::new(&a.filename));
                row.add_cell(Cell::new(&a.mime));
                row.add_cell(Cell::new(human_size(a.size)));
                row.add_cell(Cell::new(if a.inline { "yes" } else { "no" }));
                row
            }));

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}

fn human_size(bytes: usize) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}
