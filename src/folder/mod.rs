pub mod arg;
pub mod command;
pub mod config;

use color_eyre::Result;
use serde::Serialize;
use std::ops;

use crate::{
    printer::{PrintTable, PrintTableOpts, WriteColor},
    ui::{Cell, Row, Table},
};

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

#[derive(Clone, Debug, Default, Serialize)]
pub struct Folders(Vec<Folder>);

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
    fn print_table(&self, writer: &mut dyn WriteColor, opts: PrintTableOpts) -> Result<()> {
        writeln!(writer)?;
        Table::print(writer, self, opts)?;
        writeln!(writer)?;
        Ok(())
    }
}
