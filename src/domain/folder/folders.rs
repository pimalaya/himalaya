use anyhow::Result;
use serde::Serialize;
use std::ops;

use crate::{
    printer::{PrintTable, PrintTableOpts, WriteColor},
    ui::Table,
    Folder,
};

#[derive(Clone, Debug, Default, Serialize)]
pub struct Folders(Vec<Folder>);

impl ops::Deref for Folders {
    type Target = Vec<Folder>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<pimalaya_email::folder::Folders> for Folders {
    fn from(folders: pimalaya_email::folder::Folders) -> Self {
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
