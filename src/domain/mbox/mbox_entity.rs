//! Mailbox entity module.
//!
//! This module contains the definition of the mailbox and its traits implementations.

use serde::Serialize;
use std::{
    borrow::Cow,
    fmt::{self, Display},
};

use crate::{
    domain::Attrs,
    ui::{Cell, Row, Table},
};

/// Represents the mailbox.
#[derive(Debug, Default, Serialize)]
pub struct Mbox<'a> {
    /// Represents the mailbox hierarchie delimiter.
    pub delim: Cow<'a, str>,

    /// Represents the mailbox name.
    pub name: Cow<'a, str>,

    /// Represents the mailbox attributes.
    pub attrs: Attrs<'a>,
}

impl<'a> Mbox<'a> {
    /// Creates a new mailbox with just a name.
    pub fn new(name: &'a str) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }
}

/// Makes the mailbox displayable.
impl<'a> Display for Mbox<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// Makes the mailbox tableable.
impl<'a> Table for Mbox<'a> {
    fn head() -> Row {
        Row::new()
            .cell(Cell::new("DELIM").bold().underline().white())
            .cell(Cell::new("NAME").bold().underline().white())
            .cell(
                Cell::new("ATTRIBUTES")
                    .shrinkable()
                    .bold()
                    .underline()
                    .white(),
            )
    }

    fn row(&self) -> Row {
        Row::new()
            .cell(Cell::new(&self.delim).white())
            .cell(Cell::new(&self.name).green())
            .cell(Cell::new(&self.attrs.to_string()).shrinkable().blue())
    }
}

/// Converts an `imap::types::Name` into a mailbox.
impl<'a> From<&'a imap::types::Name> for Mbox<'a> {
    fn from(name: &'a imap::types::Name) -> Self {
        Self {
            delim: name.delimiter().unwrap_or_default().into(),
            name: name.name().into(),
            attrs: Attrs::from(name.attributes()),
        }
    }
}
