//! Mailbox attribute entity module.
//!
//! This module contains the definition of the mailbox attribute and its traits implementations.

pub(crate) use imap::types::NameAttribute as AttrRemote;
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

/// Represents the mailbox attribute.
/// See https://serde.rs/remote-derive.html.
#[derive(Debug, PartialEq, Eq, Hash, Serialize)]
pub struct Attr<'a>(#[serde(with = "AttrWrap")] pub &'a AttrRemote<'a>);

/// Makes the attribute displayable.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_display_attr() {
        macro_rules! attr_from {
            ($attr:ident) => {
                Attr(&AttrRemote::$attr).to_string()
            };
            ($custom:literal) => {
                Attr(&AttrRemote::Custom($custom.into())).to_string()
            };
        }

        assert_eq!("NoInferiors", attr_from![NoInferiors]);
        assert_eq!("NoSelect", attr_from![NoSelect]);
        assert_eq!("Marked", attr_from![Marked]);
        assert_eq!("Unmarked", attr_from![Unmarked]);
        assert_eq!("CustomAttr", attr_from!["CustomAttr"]);
    }
}
