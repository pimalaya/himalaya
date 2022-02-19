//! Mailbox attribute module.
//!
//! This module contains the definition of the mailbox attribute and
//! its traits implementations.

use std::{
    borrow::Cow,
    fmt::{self, Display},
    ops::Deref,
};

/// Represents the attributes of the mailbox.
#[derive(Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct MboxAttrs<'a>(pub Vec<MboxAttr<'a>>);

impl<'a> Deref for MboxAttrs<'a> {
    type Target = Vec<MboxAttr<'a>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> Display for MboxAttrs<'a> {
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
pub enum MboxAttr<'a> {
    NoInferiors,
    NoSelect,
    Marked,
    Unmarked,
    Custom(Cow<'a, str>),
}

/// Makes the attribute displayable.
impl<'a> Display for MboxAttr<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MboxAttr::NoInferiors => write!(f, "NoInferiors"),
            MboxAttr::NoSelect => write!(f, "NoSelect"),
            MboxAttr::Marked => write!(f, "Marked"),
            MboxAttr::Unmarked => write!(f, "Unmarked"),
            MboxAttr::Custom(cow) => write!(f, "{}", cow),
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
                MboxAttrs(vec![$($attr,)*]).to_string()
            };
        }

        let empty_attr = attrs_from![];
        let single_attr = attrs_from![MboxAttr::NoInferiors];
        let multiple_attrs =
            attrs_from![MboxAttr::Custom("AttrCustom".into()), MboxAttr::NoInferiors];

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
                MboxAttr::$attr.to_string()
            };
            ($custom:literal) => {
                MboxAttr::Custom($custom.into()).to_string()
            };
        }

        assert_eq!("NoInferiors", attr_from![NoInferiors]);
        assert_eq!("NoSelect", attr_from![NoSelect]);
        assert_eq!("Marked", attr_from![Marked]);
        assert_eq!("Unmarked", attr_from![Unmarked]);
        assert_eq!("CustomAttr", attr_from!["CustomAttr"]);
    }
}
