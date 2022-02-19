//! Maildir mailbox module.
//!
//! This module provides Maildir types and conversion utilities
//! related to the mailbox

use anyhow::{anyhow, Error};
use std::{
    borrow::Cow,
    convert::{TryFrom, TryInto},
    ffi::OsStr,
};

use crate::mbox::{Mbox, Mboxes};

/// Represents a list of raw mailboxes returned by the `maildir` crate.
pub type RawMaildirMboxes = Vec<RawMaildirMbox>;

impl<'a> TryFrom<&'a RawMaildirMboxes> for Mboxes<'a> {
    type Error = Error;

    fn try_from(raw_mboxes: &'a RawMaildirMboxes) -> Result<Self, Self::Error> {
        let mut mboxes = vec![];
        for raw_mbox in raw_mboxes {
            mboxes.push(raw_mbox.try_into()?);
        }
        Ok(Mboxes(mboxes))
    }
}

/// Represents the raw mailbox returned by the `maildir` crate.
pub type RawMaildirMbox = maildir::Maildir;

impl<'a> TryFrom<&'a RawMaildirMbox> for Mbox<'a> {
    type Error = Error;

    fn try_from(raw_mbox: &'a RawMaildirMbox) -> Result<Self, Self::Error> {
        let subdir_name = raw_mbox.path().file_name();
        Ok(Self {
            delim: "/".into(),
            name: subdir_name
                .and_then(OsStr::to_str)
                .and_then(|s| {
                    if s.len() < 2 {
                        None
                    } else {
                        Some(Cow::from(&s[1..]))
                    }
                })
                .ok_or_else(|| {
                    anyhow!(
                        "cannot parse maildir subdirectory name from path {:?}",
                        subdir_name,
                    )
                })?,
            ..Self::default()
        })
    }
}
