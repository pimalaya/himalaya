pub mod arg;
pub mod command;
pub mod config;

use comfy_table::{presets, Attribute, Cell, ContentArrangement, Row, Table};
use serde::{Serialize, Serializer};
use std::{fmt, ops::Deref};

#[derive(Clone, Debug, Default, Serialize)]
pub struct Folder {
    pub name: String,
    pub desc: String,
}

impl Folder {
    pub fn to_row(&self) -> Row {
        let mut row = Row::new();

        row.add_cell(Cell::new(&self.name).fg(comfy_table::Color::Blue));
        row.add_cell(Cell::new(&self.desc).fg(comfy_table::Color::Green));

        row
    }
}

impl From<email::folder::Folder> for Folder {
    fn from(folder: email::folder::Folder) -> Self {
        Folder {
            name: folder.name,
            desc: folder.desc,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Folders(Vec<Folder>);

impl Folders {
    pub fn to_table(&self) -> Table {
        let mut table = Table::new();

        table
            .load_preset(presets::NOTHING)
            .set_content_arrangement(ContentArrangement::DynamicFullWidth)
            .set_header(Row::from([
                Cell::new("NAME").add_attribute(Attribute::Reverse),
                Cell::new("DESC").add_attribute(Attribute::Reverse),
            ]))
            .add_rows(self.iter().map(Folder::to_row));

        table
    }
}

impl Deref for Folders {
    type Target = Vec<Folder>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<email::folder::Folders> for Folders {
    fn from(folders: email::folder::Folders) -> Self {
        Folders(folders.into_iter().map(Folder::from).collect())
    }
}

pub struct FoldersTable {
    folders: Folders,
    width: Option<u16>,
}

impl FoldersTable {
    pub fn with_some_width(mut self, width: Option<u16>) -> Self {
        self.width = width;
        self
    }
}

impl From<Folders> for FoldersTable {
    fn from(folders: Folders) -> Self {
        Self {
            folders,
            width: None,
        }
    }
}

impl fmt::Display for FoldersTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = self.folders.to_table();

        if let Some(width) = self.width {
            table.set_width(width);
        }

        writeln!(f)?;
        write!(f, "{table}")?;
        writeln!(f)?;
        Ok(())
    }
}

impl Serialize for FoldersTable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.folders.serialize(serializer)
    }
}
