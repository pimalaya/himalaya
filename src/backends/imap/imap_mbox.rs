//! IMAP mailbox module.
//!
//! This module provides IMAP types and conversion utilities related
//! to the mailbox.

use anyhow::Result;
use std::fmt::{self, Display};
use std::ops::Deref;

use crate::mbox::Mboxes;
use crate::{
    output::{PrintTable, PrintTableOpts, WriteColor},
    ui::{Cell, Row, Table},
};

use super::ImapMboxAttrs;

/// Represents a list of IMAP mailboxes.
#[derive(Debug, Default, serde::Serialize)]
pub struct ImapMboxes(pub Vec<ImapMbox>);

impl Deref for ImapMboxes {
    type Target = Vec<ImapMbox>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PrintTable for ImapMboxes {
    fn print_table(&self, writter: &mut dyn WriteColor, opts: PrintTableOpts) -> Result<()> {
        writeln!(writter)?;
        Table::print(writter, self, opts)?;
        writeln!(writter)?;
        Ok(())
    }
}

impl Mboxes for ImapMboxes {
    //
}

/// Represents the IMAP mailbox.
#[derive(Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct ImapMbox {
    /// Represents the mailbox hierarchie delimiter.
    pub delim: String,

    /// Represents the mailbox name.
    pub name: String,

    /// Represents the mailbox attributes.
    pub attrs: ImapMboxAttrs,
}

impl ImapMbox {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }
}

impl Display for ImapMbox {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Table for ImapMbox {
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
    use crate::backends::ImapMboxAttr;

    use super::*;

    #[test]
    fn it_should_create_new_mbox() {
        assert_eq!(ImapMbox::default(), ImapMbox::new(""));
        assert_eq!(
            ImapMbox {
                name: "INBOX".into(),
                ..ImapMbox::default()
            },
            ImapMbox::new("INBOX")
        );
    }

    #[test]
    fn it_should_display_mbox() {
        let default_mbox = ImapMbox::default();
        assert_eq!("", default_mbox.to_string());

        let new_mbox = ImapMbox::new("INBOX");
        assert_eq!("INBOX", new_mbox.to_string());

        let full_mbox = ImapMbox {
            delim: ".".into(),
            name: "Sent".into(),
            attrs: ImapMboxAttrs(vec![ImapMboxAttr::NoSelect]),
        };
        assert_eq!("Sent", full_mbox.to_string());
    }
}

/// Represents a list of raw mailboxes returned by the `imap` crate.
pub type RawImapMboxes = imap::types::ZeroCopy<Vec<RawImapMbox>>;

impl<'a> From<RawImapMboxes> for ImapMboxes {
    fn from(raw_mboxes: RawImapMboxes) -> Self {
        Self(raw_mboxes.iter().map(ImapMbox::from).collect())
    }
}

/// Represents the raw mailbox returned by the `imap` crate.
pub type RawImapMbox = imap::types::Name;

impl<'a> From<&'a RawImapMbox> for ImapMbox {
    fn from(raw_mbox: &'a RawImapMbox) -> Self {
        Self {
            delim: raw_mbox.delimiter().unwrap_or_default().into(),
            name: raw_mbox.name().into(),
            attrs: raw_mbox.attributes().into(),
        }
    }
}
