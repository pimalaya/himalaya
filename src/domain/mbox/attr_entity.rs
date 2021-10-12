//! Mailbox attribute entity module.
//!
//! This module contains the definition of the mailbox attribute and its traits implementations.

use imap::types::NameAttribute as AttrRemote;
use serde::Serialize;
use std::{
    borrow::Cow,
    fmt::{self, Display},
};

/// Wraps an `imap::types::NameAttribute`.
/// See https://serde.rs/remote-derive.html.
#[derive(Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(remote = "AttrRemote")]
pub enum AttrWrap<'a> {
    NoInferiors,
    NoSelect,
    Marked,
    Unmarked,
    Custom(Cow<'a, str>),
}

/// Represent a mailbox attribute.
/// See https://serde.rs/remote-derive.html.
#[derive(Debug, PartialEq, Eq, Hash, Serialize)]
pub struct Attr<'a>(#[serde(with = "AttrWrap")] pub &'a AttrRemote<'a>);

/// Makes attribute displayable.
impl<'a> Display for Attr<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.0 {
            AttrRemote::NoInferiors => write!(f, "NoInferiors"),
            AttrRemote::NoSelect => write!(f, "NoSelect"),
            AttrRemote::Marked => write!(f, "Marked"),
            AttrRemote::Unmarked => write!(f, "Unmarked"),
            AttrRemote::Custom(cow) => write!(f, "{}", cow),
        }
    }
}
