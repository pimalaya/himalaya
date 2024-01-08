pub mod arg;
#[cfg(feature = "folder-subcmd")]
pub mod command;
pub mod config;

#[cfg(feature = "folder-subcmd")]
use anyhow::Result;
#[cfg(feature = "folder-subcmd")]
use serde::Serialize;
#[cfg(feature = "folder-subcmd")]
use std::ops;

#[cfg(feature = "folder-subcmd")]
use crate::{
    printer::{PrintTable, PrintTableOpts, WriteColor},
    ui::{Cell, Row, Table},
};

#[cfg(feature = "folder-subcmd")]
#[derive(Clone, Debug, Default, Serialize)]
pub struct Folder {
    pub name: String,
    pub desc: String,
}

#[cfg(feature = "folder-subcmd")]
impl From<&email::folder::Folder> for Folder {
    fn from(folder: &email::folder::Folder) -> Self {
        Folder {
            name: folder.name.clone(),
            desc: folder.desc.clone(),
        }
    }
}

#[cfg(feature = "folder-subcmd")]
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

#[cfg(feature = "folder-subcmd")]
#[derive(Clone, Debug, Default, Serialize)]
pub struct Folders(Vec<Folder>);

#[cfg(feature = "folder-subcmd")]
impl ops::Deref for Folders {
    type Target = Vec<Folder>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "folder-subcmd")]
impl From<email::folder::Folders> for Folders {
    fn from(folders: email::folder::Folders) -> Self {
        Folders(folders.iter().map(Folder::from).collect())
    }
}

#[cfg(feature = "folder-subcmd")]
impl PrintTable for Folders {
    fn print_table(&self, writer: &mut dyn WriteColor, opts: PrintTableOpts) -> Result<()> {
        writeln!(writer)?;
        Table::print(writer, self, opts)?;
        writeln!(writer)?;
        Ok(())
    }
}
