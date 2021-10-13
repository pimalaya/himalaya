//! Mailbox attributes entity module.
//!
//! This module contains the definition of the mailbox attributes and its traits implementations.

use imap::types::NameAttribute as AttrRemote;
use serde::Serialize;
use std::{
    collections::HashSet,
    fmt::{self, Display},
    ops::Deref,
};

use crate::domain::Attr;

/// Represents the attributes of the mailbox.
/// A HashSet is used in order to avoid duplicates.
#[derive(Debug, Default, Serialize)]
pub struct Attrs<'a>(HashSet<Attr<'a>>);

/// Converts a slice of `imap::types::NameAttribute` into attributes.
impl<'a> From<&'a [AttrRemote<'a>]> for Attrs<'a> {
    fn from(attrs: &'a [AttrRemote<'a>]) -> Self {
        Self(attrs.iter().map(Attr).collect())
    }
}

/// Derefs the attributes to its inner hashset.
impl<'a> Deref for Attrs<'a> {
    type Target = HashSet<Attr<'a>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Makes the attributes displayable.
impl<'a> Display for Attrs<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut glue = "";
        for attr in self.iter() {
            write!(f, "{}{}", glue, attr)?;
            glue = ", ";
        }
        Ok(())
    }
}
