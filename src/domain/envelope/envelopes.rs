use std::ops;

use anyhow::Result;
use serde::Serialize;

use crate::{
    printer::{PrintTable, PrintTableOpts, WriteColor},
    ui::Table,
    Envelope,
};

/// Represents the list of envelopes.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Envelopes(Vec<Envelope>);

impl ops::Deref for Envelopes {
    type Target = Vec<Envelope>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<himalaya_lib::Envelopes> for Envelopes {
    fn from(envelopes: himalaya_lib::Envelopes) -> Self {
        Envelopes(envelopes.iter().map(Envelope::from).collect())
    }
}

impl PrintTable for Envelopes {
    fn print_table(&self, writer: &mut dyn WriteColor, opts: PrintTableOpts) -> Result<()> {
        writeln!(writer)?;
        Table::print(writer, self, opts)?;
        writeln!(writer)?;
        Ok(())
    }
}
