use anyhow::Result;
use himalaya_lib::mbox::Mboxes;

use crate::{
    output::{PrintTable, PrintTableOpts, WriteColor},
    ui::Table,
};

impl PrintTable for Mboxes {
    fn print_table(&self, writer: &mut dyn WriteColor, opts: PrintTableOpts) -> Result<()> {
        writeln!(writer)?;
        Table::print(writer, self, opts)?;
        writeln!(writer)?;
        Ok(())
    }
}
