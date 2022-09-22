use anyhow::Result;
use himalaya_lib::folder::Folders;

use crate::{
    printer::{PrintTable, PrintTableOpts, WriteColor},
    ui::Table,
};

impl PrintTable for Folders {
    fn print_table(&self, writer: &mut dyn WriteColor, opts: PrintTableOpts) -> Result<()> {
        writeln!(writer)?;
        Table::print(writer, self, opts)?;
        writeln!(writer)?;
        Ok(())
    }
}
