//! Mailbox attributes entity module.
//!
//! This module contains the definition of the mailbox attributes and its traits implementations.

use serde::Serialize;
use std::{
    collections::HashSet,
    fmt::{self, Display},
    ops::Deref,
};

use crate::domain::{Attr, AttrRemote};

/// Represents the attributes of the mailbox.
/// A HashSet is used in order to avoid duplicates.
#[derive(Debug, Default, PartialEq, Eq, Serialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_display_attrs() {
        macro_rules! attrs_from {
            ($($attr:expr),*) => {
                Attrs::from(&[$($attr,)*] as &[AttrRemote]).to_string()
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
}
