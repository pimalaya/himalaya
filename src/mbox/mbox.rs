//! Mailbox module.
//!
//! This module contains the definition of the mailbox and its traits
//! implementations.

use anyhow::Result;
use std::fmt::{self, Display};
use std::ops::Deref;

use crate::{
    mbox::MboxAttrs,
    output::{PrintTable, PrintTableOpts, WriteColor},
    ui::{Cell, Row, Table},
};

/// Represents a list of mailboxes.
#[derive(Debug, Default, serde::Serialize)]
pub struct Mboxes(pub Vec<Mbox>);

impl Deref for Mboxes {
    type Target = Vec<Mbox>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PrintTable for Mboxes {
    fn print_table(&self, writter: &mut dyn WriteColor, opts: PrintTableOpts) -> Result<()> {
        writeln!(writter)?;
        Table::print(writter, self, opts)?;
        writeln!(writter)?;
        Ok(())
    }
}

/// Represents the mailbox.
#[derive(Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct Mbox {
    /// Represents the mailbox hierarchie delimiter.
    pub delim: String,

    /// Represents the mailbox name.
    pub name: String,

    /// Represents the mailbox attributes.
    pub attrs: MboxAttrs,
}

impl Mbox {
    /// Creates a new mailbox with just a name.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }
}

impl Display for Mbox {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Table for Mbox {
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

#[cfg(test)]
mod tests {
    use crate::mbox::{MboxAttr, MboxAttrs};

    use super::*;

    #[test]
    fn it_should_create_new_mbox() {
        assert_eq!(Mbox::default(), Mbox::new(""));
        assert_eq!(
            Mbox {
                name: "INBOX".into(),
                ..Mbox::default()
            },
            Mbox::new("INBOX")
        );
    }

    #[test]
    fn it_should_display_mbox() {
        let default_mbox = Mbox::default();
        assert_eq!("", default_mbox.to_string());

        let new_mbox = Mbox::new("INBOX");
        assert_eq!("INBOX", new_mbox.to_string());

        let full_mbox = Mbox {
            delim: ".".into(),
            name: "Sent".into(),
            attrs: MboxAttrs(vec![MboxAttr::NoSelect]),
        };
        assert_eq!("Sent", full_mbox.to_string());
    }
}
