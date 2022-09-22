use anyhow::Result;
use himalaya_lib::Envelopes;

use crate::{
    printer::{PrintTable, PrintTableOpts, WriteColor},
    ui::Table,
};

impl PrintTable for Envelopes {
    fn print_table(&self, writer: &mut dyn WriteColor, opts: PrintTableOpts) -> Result<()> {
        writeln!(writer)?;
        Table::print(writer, self, opts)?;
        writeln!(writer)?;
        Ok(())
    }
}
