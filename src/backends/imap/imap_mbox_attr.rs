//! IMAP mailbox attribute module.
//!
//! This module provides IMAP types and conversion utilities related
//! to the mailbox attribute.

/// Represents the raw mailbox attribute returned by the `imap` crate.
pub use imap::types::NameAttribute as RawImapMboxAttr;
use std::{
    fmt::{self, Display},
    ops::Deref,
};

/// Represents the attributes of the mailbox.
#[derive(Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct ImapMboxAttrs(pub Vec<ImapMboxAttr>);

impl Deref for ImapMboxAttrs {
    type Target = Vec<ImapMboxAttr>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for ImapMboxAttrs {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut glue = "";
        for attr in self.iter() {
            write!(f, "{}{}", glue, attr)?;
            glue = ", ";
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
pub enum ImapMboxAttr {
    NoInferiors,
    NoSelect,
    Marked,
    Unmarked,
    Custom(String),
}

/// Makes the attribute displayable.
impl Display for ImapMboxAttr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ImapMboxAttr::NoInferiors => write!(f, "NoInferiors"),
            ImapMboxAttr::NoSelect => write!(f, "NoSelect"),
            ImapMboxAttr::Marked => write!(f, "Marked"),
            ImapMboxAttr::Unmarked => write!(f, "Unmarked"),
            ImapMboxAttr::Custom(custom) => write!(f, "{}", custom),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_display_attrs() {
        macro_rules! attrs_from {
            ($($attr:expr),*) => {
                ImapMboxAttrs(vec![$($attr,)*]).to_string()
            };
        }

        let empty_attr = attrs_from![];
        let single_attr = attrs_from![ImapMboxAttr::NoInferiors];
        let multiple_attrs = attrs_from![
            ImapMboxAttr::Custom("AttrCustom".into()),
            ImapMboxAttr::NoInferiors
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
                ImapMboxAttr::$attr.to_string()
            };
            ($custom:literal) => {
                ImapMboxAttr::Custom($custom.into()).to_string()
            };
        }

        assert_eq!("NoInferiors", attr_from![NoInferiors]);
        assert_eq!("NoSelect", attr_from![NoSelect]);
        assert_eq!("Marked", attr_from![Marked]);
        assert_eq!("Unmarked", attr_from![Unmarked]);
        assert_eq!("CustomAttr", attr_from!["CustomAttr"]);
    }
}

impl<'a> From<&'a [RawImapMboxAttr<'a>]> for ImapMboxAttrs {
    fn from(raw_attrs: &'a [RawImapMboxAttr<'a>]) -> Self {
        Self(raw_attrs.iter().map(ImapMboxAttr::from).collect())
    }
}

impl<'a> From<&'a RawImapMboxAttr<'a>> for ImapMboxAttr {
    fn from(attr: &'a RawImapMboxAttr<'a>) -> Self {
        match attr {
            RawImapMboxAttr::NoInferiors => Self::NoInferiors,
            RawImapMboxAttr::NoSelect => Self::NoSelect,
            RawImapMboxAttr::Marked => Self::Marked,
            RawImapMboxAttr::Unmarked => Self::Unmarked,
            RawImapMboxAttr::Custom(cow) => Self::Custom(cow.to_string()),
        }
    }
}
