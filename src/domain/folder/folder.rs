use serde::Serialize;

use crate::ui::{Cell, Row, Table};

#[derive(Clone, Debug, Default, Serialize)]
pub struct Folder {
    pub name: String,
    pub desc: String,
}

impl From<&pimalaya_email::folder::Folder> for Folder {
    fn from(folder: &pimalaya_email::folder::Folder) -> Self {
        Folder {
            name: folder.name.clone(),
            desc: folder.desc.clone(),
        }
    }
}

impl Table for Folder {
    fn head() -> Row {
        Row::new()
            .cell(Cell::new("NAME").bold().underline().white())
            .cell(Cell::new("DESC").bold().underline().white())
    }

    fn row(&self) -> Row {
        Row::new()
            .cell(Cell::new(&self.name).blue())
            .cell(Cell::new(&self.desc).green())
    }
}
