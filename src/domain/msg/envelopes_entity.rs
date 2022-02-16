use anyhow::{Error, Result};
use serde::Serialize;
use std::{convert::TryFrom, ops::Deref};

use crate::{
    domain::{msg::Envelope, RawEnvelope},
    output::{PrintTable, PrintTableOpts, WriteColor},
    ui::Table,
};

pub type RawEnvelopes = imap::types::ZeroCopy<Vec<RawEnvelope>>;

/// Representation of a list of envelopes.
#[derive(Debug, Default, Clone, Serialize)]
pub struct Envelopes<'a>(pub Vec<Envelope<'a>>);

impl<'a> From<Vec<Envelope<'a>>> for Envelopes<'a> {
    fn from(envelopes: Vec<Envelope<'a>>) -> Self {
        Self(envelopes)
    }
}

impl<'a> From<&'a [Envelope<'a>]> for Envelopes<'a> {
    fn from(envelopes: &'a [Envelope<'a>]) -> Self {
        Self(envelopes.to_vec())
    }
}

impl<'a> Deref for Envelopes<'a> {
    type Target = Vec<Envelope<'a>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> TryFrom<&'a RawEnvelopes> for Envelopes<'a> {
    type Error = Error;

    fn try_from(fetches: &'a RawEnvelopes) -> Result<Self> {
        let mut envelopes = vec![];

        for fetch in fetches.iter().rev() {
            envelopes.push(Envelope::try_from(fetch)?);
        }

        Ok(Self(envelopes))
    }
}

impl<'a> PrintTable for Envelopes<'a> {
    fn print_table(&self, writter: &mut dyn WriteColor, opts: PrintTableOpts) -> Result<()> {
        writeln!(writter)?;
        Table::print(writter, self, opts)?;
        writeln!(writter)?;
        Ok(())
    }
}
