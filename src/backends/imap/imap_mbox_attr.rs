//! IMAP mailbox attribute module.
//!
//! This module provides IMAP types and conversion utilities related
//! to the mailbox attribute.

use std::borrow::Cow;

/// Represents the raw mailbox attribute returned by the `imap` crate.
pub use imap::types::NameAttribute as RawImapMboxAttr;

use crate::mbox::{MboxAttr, MboxAttrs};

impl<'a> From<&'a [RawImapMboxAttr<'a>]> for MboxAttrs<'a> {
    fn from(raw_attrs: &'a [RawImapMboxAttr<'a>]) -> Self {
        Self(raw_attrs.iter().map(MboxAttr::from).collect())
    }
}

impl<'a> From<&'a RawImapMboxAttr<'a>> for MboxAttr<'a> {
    fn from(attr: &'a RawImapMboxAttr<'a>) -> Self {
        match attr {
            RawImapMboxAttr::NoInferiors => Self::NoInferiors,
            RawImapMboxAttr::NoSelect => Self::NoSelect,
            RawImapMboxAttr::Marked => Self::Marked,
            RawImapMboxAttr::Unmarked => Self::Unmarked,
            RawImapMboxAttr::Custom(cow) => Self::Custom(Cow::Borrowed(cow)),
        }
    }
}
