//! Notmuch mailbox module.
//!
//! This module provides Notmuch types and conversion utilities
//! related to the mailbox

use anyhow::Result;
use std::{
    fmt::{self, Display},
    ops::Deref,
};

use crate::{
    mbox::Mboxes,
    output::{PrintTable, PrintTableOpts, WriteColor},
    ui::{Cell, Row, Table},
};

/// Represents a list of Notmuch mailboxes.
#[derive(Debug, Default, serde::Serialize)]
pub struct NotmuchMboxes(pub Vec<NotmuchMbox>);

impl Deref for NotmuchMboxes {
    type Target = Vec<NotmuchMbox>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PrintTable for NotmuchMboxes {
    fn print_table(&self, writer: &mut dyn WriteColor, opts: PrintTableOpts) -> Result<()> {
        writeln!(writer)?;
        Table::print(writer, self, opts)?;
        writeln!(writer)?;
        Ok(())
    }
}

impl Mboxes for NotmuchMboxes {
    //
}

/// Represents the notmuch virtual mailbox.
#[derive(Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct NotmuchMbox {
    /// Represents the virtual mailbox name.
    pub name: String,

    /// Represents the query associated to the virtual mailbox name.
    pub query: String,
}

impl NotmuchMbox {
    pub fn new(name: &str, query: &str) -> Self {
        Self {
            name: name.into(),
            query: query.into(),
        }
    }
}

impl Display for NotmuchMbox {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Table for NotmuchMbox {
    fn head() -> Row {
        Row::new()
            .cell(Cell::new("NAME").bold().underline().white())
            .cell(Cell::new("QUERY").bold().underline().white())
    }

    fn row(&self) -> Row {
        Row::new()
            .cell(Cell::new(&self.name).white())
            .cell(Cell::new(&self.query).green())
    }
}
