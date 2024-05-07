pub mod arg;
pub mod command;
pub mod config;

use color_eyre::Result;
use comfy_table::{presets, Attribute, Cell, ContentArrangement, Row, Table};
use serde::Serialize;
use std::ops;

use crate::printer::{PrintTable, WriteColor};

#[derive(Clone, Debug, Default, Serialize)]
pub struct Folder {
    pub name: String,
    pub desc: String,
}

impl From<&email::folder::Folder> for Folder {
    fn from(folder: &email::folder::Folder) -> Self {
        Folder {
            name: folder.name.clone(),
            desc: folder.desc.clone(),
        }
    }
}
impl From<&Folder> for Row {
    fn from(folder: &Folder) -> Self {
        let mut row = Row::new();
        row.add_cell(Cell::new(&folder.name).fg(comfy_table::Color::Blue));
        row.add_cell(Cell::new(&folder.desc).fg(comfy_table::Color::Green));

        row
    }
}

impl From<Folder> for Row {
    fn from(folder: Folder) -> Self {
        let mut row = Row::new();
        row.add_cell(Cell::new(folder.name).fg(comfy_table::Color::Blue));
        row.add_cell(Cell::new(folder.desc).fg(comfy_table::Color::Green));

        row
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Folders(Vec<Folder>);

impl From<Folders> for Table {
    fn from(folders: Folders) -> Self {
        let mut table = Table::new();
        table
            .load_preset(presets::NOTHING)
            .set_header(Row::from([
                Cell::new("NAME").add_attribute(Attribute::Reverse),
                Cell::new("DESC").add_attribute(Attribute::Reverse),
            ]))
            .add_rows(folders.0.into_iter().map(Row::from));
        table
    }
}

impl From<&Folders> for Table {
    fn from(folders: &Folders) -> Self {
        let mut table = Table::new();
        table
            .load_preset(presets::NOTHING)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(Row::from([
                Cell::new("NAME").add_attribute(Attribute::Reverse),
                Cell::new("DESC").add_attribute(Attribute::Reverse),
            ]))
            .add_rows(folders.0.iter().map(Row::from));
        table
    }
}

impl ops::Deref for Folders {
    type Target = Vec<Folder>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<email::folder::Folders> for Folders {
    fn from(folders: email::folder::Folders) -> Self {
        Folders(folders.iter().map(Folder::from).collect())
    }
}

impl PrintTable for Folders {
    fn print_table(&self, writer: &mut dyn WriteColor, table_max_width: Option<u16>) -> Result<()> {
        let mut table = Table::from(self);
        if let Some(width) = table_max_width {
            table.set_width(width);
        }
        writeln!(writer)?;
        write!(writer, "{}", table)?;
        writeln!(writer)?;
        Ok(())
    }
}
