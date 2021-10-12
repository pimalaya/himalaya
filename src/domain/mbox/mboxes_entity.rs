//! Mailboxes entity module.
//!
//! This module contains the definition of the mailboxes and its traits implementations.

use serde::Serialize;
use std::{
    fmt::{self, Display},
    ops::Deref,
};

use crate::{domain::Mbox, ui::Table};

/// Represents a list of mailboxes.
#[derive(Debug, Serialize)]
pub struct Mboxes<'a>(Vec<Mbox<'a>>);

/// Makes the mailboxes derefable.
impl<'a> Deref for Mboxes<'a> {
    type Target = Vec<Mbox<'a>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Makes the mailboxes displayable.
impl<'a> Display for Mboxes<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "\n{}", Table::render(&self))
    }
}

/// Converts a list of `imap::types::Name` into mailboxes.
impl<'a> From<&'a imap::types::ZeroCopy<Vec<imap::types::Name>>> for Mboxes<'a> {
    fn from(names: &'a imap::types::ZeroCopy<Vec<imap::types::Name>>) -> Self {
        Self(names.iter().map(Mbox::from).collect::<Vec<_>>())
    }
}
