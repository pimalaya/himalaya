use himalaya_lib::mbox::Mbox;

use crate::ui::{Cell, Row, Table};

impl Table for Mbox {
    fn head() -> Row {
        Row::new()
            .cell(Cell::new("DELIM").bold().underline().white())
            .cell(Cell::new("NAME").bold().underline().white())
            .cell(Cell::new("DESC").bold().underline().white())
    }

    fn row(&self) -> Row {
        Row::new()
            .cell(Cell::new(&self.delim).white())
            .cell(Cell::new(&self.name).blue())
            .cell(Cell::new(&self.desc).green())
    }
}
