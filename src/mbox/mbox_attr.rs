//! Mailbox attribute module.
//!
//! This module contains the definition of the mailbox attribute and
//! its traits implementations.

pub use imap::types::NameAttribute as AttrRemote;
use serde::Serialize;
use std::{
    borrow::Cow,
    fmt::{self, Display},
    ops::Deref,
};

/// Represents the attributes of the mailbox.
#[derive(Debug, Default, PartialEq, Eq, Serialize)]
pub struct Attrs<'a>(Vec<Attr<'a>>);

/// Converts a vector of `imap::types::NameAttribute` into attributes.
impl<'a> From<Vec<AttrRemote<'a>>> for Attrs<'a> {
    fn from(attrs: Vec<AttrRemote<'a>>) -> Self {
        Self(attrs.into_iter().map(Attr::from).collect())
    }
}

/// Derefs the attributes to its inner hashset.
impl<'a> Deref for Attrs<'a> {
    type Target = Vec<Attr<'a>>;

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
pub struct Attr<'a>(#[serde(with = "AttrWrap")] pub AttrRemote<'a>);

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

/// Converts an `imap::types::NameAttribute` into an attribute.
impl<'a> From<AttrRemote<'a>> for Attr<'a> {
    fn from(attr: AttrRemote<'a>) -> Self {
        Self(attr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_display_attrs() {
        macro_rules! attrs_from {
            ($($attr:expr),*) => {
                Attrs::from(vec![$($attr,)*]).to_string()
            };
        }

        let empty_attr = attrs_from![];
        let single_attr = attrs_from![AttrRemote::NoInferiors];
        let multiple_attrs = attrs_from![
            AttrRemote::Custom("AttrCustom".into()),
            AttrRemote::NoInferiors
        ];

        assert_eq!("", empty_attr);
        assert_eq!("NoInferiors", single_attr);
        assert!(multiple_attrs.contains("NoInferiors"));
        assert!(multiple_attrs.contains("AttrCustom"));
        assert!(multiple_attrs.contains(","));
    }

    #[test]
    fn it_should_display_attr() {
        macro_rules! attr_from {
            ($attr:ident) => {
                Attr(AttrRemote::$attr).to_string()
            };
            ($custom:literal) => {
                Attr(AttrRemote::Custom($custom.into())).to_string()
            };
        }

        assert_eq!("NoInferiors", attr_from![NoInferiors]);
        assert_eq!("NoSelect", attr_from![NoSelect]);
        assert_eq!("Marked", attr_from![Marked]);
        assert_eq!("Unmarked", attr_from![Unmarked]);
        assert_eq!("CustomAttr", attr_from!["CustomAttr"]);
    }
}
