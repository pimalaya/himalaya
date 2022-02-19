//! Envelope module.
//!
//! This module contains the definition of the envelope and its traits
//! implementations.

use anyhow::Result;
use std::{borrow::Cow, ops::Deref};

use crate::{
    domain::msg::{Flag, Flags},
    output::{PrintTable, PrintTableOpts, WriteColor},
    ui::{Cell, Row, Table},
};

/// Represents a list of envelopes.
#[derive(Debug, Default, serde::Serialize)]
pub struct Envelopes<'a>(pub Vec<Envelope<'a>>);

impl<'a> Deref for Envelopes<'a> {
    type Target = Vec<Envelope<'a>>;

    fn deref(&self) -> &Self::Target {
        &self.0
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

/// Represents the envelope. The envelope is just a message subset,
/// and is mostly used for listings.
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct Envelope<'a> {
    /// Represents the sequence number of the message.
    ///
    /// [RFC3501]: https://datatracker.ietf.org/doc/html/rfc3501#section-2.3.1.2
    pub id: u32,

    /// Represents the flags attached to the message.
    pub flags: Flags,

    /// Represents the subject of the message.
    pub subject: Cow<'a, str>,

    /// Represents the sender of the message.
    pub sender: String,

    /// Represents the internal date of the message.
    ///
    /// [RFC3501]: https://datatracker.ietf.org/doc/html/rfc3501#section-2.3.3
    pub date: Option<String>,
}

impl<'a> Table for Envelope<'a> {
    fn head() -> Row {
        Row::new()
            .cell(Cell::new("ID").bold().underline().white())
            .cell(Cell::new("FLAGS").bold().underline().white())
            .cell(Cell::new("SUBJECT").shrinkable().bold().underline().white())
            .cell(Cell::new("SENDER").bold().underline().white())
            .cell(Cell::new("DATE").bold().underline().white())
    }

    fn row(&self) -> Row {
        let id = self.id.to_string();
        let flags = self.flags.to_symbols_string();
        let unseen = !self.flags.contains(&Flag::Seen);
        let subject = &self.subject;
        let sender = &self.sender;
        let date = self.date.as_deref().unwrap_or_default();
        Row::new()
            .cell(Cell::new(id).bold_if(unseen).red())
            .cell(Cell::new(flags).bold_if(unseen).white())
            .cell(Cell::new(subject).shrinkable().bold_if(unseen).green())
            .cell(Cell::new(sender).bold_if(unseen).blue())
            .cell(Cell::new(date).bold_if(unseen).yellow())
    }
}
