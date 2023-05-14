use anyhow::Result;
use serde::Serialize;
use std::ops;

use crate::{
    printer::{PrintTable, PrintTableOpts, WriteColor},
    ui::Table,
    Envelope, IdMapper,
};

/// Represents the list of envelopes.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Envelopes(Vec<Envelope>);

impl Envelopes {
    pub fn remap_ids(&mut self, id_mapper: &IdMapper) -> Result<()> {
        for envelope in &mut self.0 {
            envelope.id = id_mapper.get_or_create_alias(&envelope.id)?;
        }
        Ok(())
    }
}

impl ops::Deref for Envelopes {
    type Target = Vec<Envelope>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<pimalaya_email::Envelopes> for Envelopes {
    fn from(envelopes: pimalaya_email::Envelopes) -> Self {
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
