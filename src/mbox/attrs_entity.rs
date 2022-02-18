//! Mailbox attributes entity module.
//!
//! This module contains the definition of the mailbox attributes and its traits implementations.

use serde::Serialize;
use std::{
    fmt::{self, Display},
    ops::Deref,
};

use crate::{Attr, AttrRemote};

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
}
