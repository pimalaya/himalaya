//! Mailboxes entity module.
//!
//! This module contains the definition of the mailboxes and its traits implementations.

use anyhow::Result;
use serde::Serialize;
use std::ops::Deref;

use crate::{
    domain::{Mbox, RawMbox},
    output::{PrintTable, PrintTableOpts, WriteColor},
    ui::Table,
};

/// Represents a list of raw mailboxes returned by the `imap` crate.
pub(crate) type RawMboxes = imap::types::ZeroCopy<Vec<RawMbox>>;

/// Represents a list of mailboxes.
#[derive(Debug, Default, Serialize)]
pub struct Mboxes<'a>(pub Vec<Mbox<'a>>);

/// Derefs the mailboxes to its inner vector.
impl<'a> Deref for Mboxes<'a> {
    type Target = Vec<Mbox<'a>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Makes the mailboxes printable.
impl<'a> PrintTable for Mboxes<'a> {
    fn print_table(&self, writter: &mut dyn WriteColor, opts: PrintTableOpts) -> Result<()> {
        writeln!(writter)?;
        Table::print(writter, self, opts)?;
        writeln!(writter)?;
        Ok(())
    }
}

/// Converts a list of `imap::types::Name` into mailboxes.
impl<'a> From<&'a RawMboxes> for Mboxes<'a> {
    fn from(raw_mboxes: &'a RawMboxes) -> Mboxes<'a> {
        Self(raw_mboxes.iter().map(Mbox::from).collect())
    }
}
