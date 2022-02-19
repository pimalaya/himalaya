//! IMAP mailbox module.
//!
//! This module provides IMAP types and conversion utilities related
//! to the mailbox.

use crate::mbox::{Mbox, Mboxes};

/// Represents a list of raw mailboxes returned by the `imap` crate.
pub type RawImapMboxes = imap::types::ZeroCopy<Vec<RawImapMbox>>;

impl<'a> From<&'a RawImapMboxes> for Mboxes<'a> {
    fn from(raw_mboxes: &'a RawImapMboxes) -> Self {
        Self(raw_mboxes.iter().map(Mbox::from).collect())
    }
}

/// Represents the raw mailbox returned by the `imap` crate.
pub type RawImapMbox = imap::types::Name;

impl<'a> From<&'a RawImapMbox> for Mbox<'a> {
    fn from(raw_mbox: &'a RawImapMbox) -> Self {
        Self {
            delim: raw_mbox.delimiter().unwrap_or_default().into(),
            name: raw_mbox.name().into(),
            attrs: raw_mbox.attributes().into(),
        }
    }
}
