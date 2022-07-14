//! Mailbox module.
//!
//! This module contains the representation of the mailbox.

use serde::Serialize;
use std::fmt;

/// Represents the mailbox.
#[derive(Debug, Default, PartialEq, Eq, Serialize)]
pub struct Mbox {
    /// Represents the mailbox hierarchie delimiter.
    pub delim: String,
    /// Represents the mailbox name.
    pub name: String,
    /// Represents the mailbox description.
    pub desc: String,
}

impl fmt::Display for Mbox {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}
