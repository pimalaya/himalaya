//! Maildir mailbox module.
//!
//! This module provides Maildir types and conversion utilities
//! related to the mailbox

use anyhow::{anyhow, Error, Result};
use std::{
    convert::{TryFrom, TryInto},
    ffi::OsStr,
    fmt::{self, Display},
    ops::Deref,
};

use crate::{
    mbox::Mboxes,
    output::{PrintTable, PrintTableOpts, WriteColor},
    ui::{Cell, Row, Table},
};

/// Represents a list of Maildir mailboxes.
#[derive(Debug, Default, serde::Serialize)]
pub struct MaildirMboxes {
    #[serde(rename = "response")]
    pub mboxes: Vec<MaildirMbox>,
}

impl Deref for MaildirMboxes {
    type Target = Vec<MaildirMbox>;

    fn deref(&self) -> &Self::Target {
        &self.mboxes
    }
}

impl PrintTable for MaildirMboxes {
    fn print_table(&self, writer: &mut dyn WriteColor, opts: PrintTableOpts) -> Result<()> {
        writeln!(writer)?;
        Table::print(writer, self, opts)?;
        writeln!(writer)?;
        Ok(())
    }
}

impl Mboxes for MaildirMboxes {
    //
}

/// Represents the mailbox.
#[derive(Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct MaildirMbox {
    /// Represents the mailbox name.
    pub name: String,
}

impl MaildirMbox {
    pub fn new(name: &str) -> Self {
        Self { name: name.into() }
    }
}

impl Display for MaildirMbox {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Table for MaildirMbox {
    fn head() -> Row {
        Row::new().cell(Cell::new("SUBDIR").bold().underline().white())
    }

    fn row(&self) -> Row {
        Row::new().cell(Cell::new(&self.name).green())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_create_new_mbox() {
        assert_eq!(MaildirMbox::default(), MaildirMbox::new(""));
        assert_eq!(
            MaildirMbox {
                name: "INBOX".into(),
                ..MaildirMbox::default()
            },
            MaildirMbox::new("INBOX")
        );
    }

    #[test]
    fn it_should_display_mbox() {
        let default_mbox = MaildirMbox::default();
        assert_eq!("", default_mbox.to_string());

        let new_mbox = MaildirMbox::new("INBOX");
        assert_eq!("INBOX", new_mbox.to_string());

        let full_mbox = MaildirMbox {
            name: "Sent".into(),
        };
        assert_eq!("Sent", full_mbox.to_string());
    }
}

/// Represents a list of raw mailboxes returned by the `maildir` crate.
pub type RawMaildirMboxes = maildir::MaildirEntries;

impl TryFrom<RawMaildirMboxes> for MaildirMboxes {
    type Error = Error;

    fn try_from(mail_entries: RawMaildirMboxes) -> Result<Self, Self::Error> {
        let mut mboxes = vec![];
        for entry in mail_entries {
            mboxes.push(entry?.try_into()?);
        }
        Ok(MaildirMboxes { mboxes })
    }
}

/// Represents the raw mailbox returned by the `maildir` crate.
pub type RawMaildirMbox = maildir::Maildir;

impl TryFrom<RawMaildirMbox> for MaildirMbox {
    type Error = Error;

    fn try_from(mail_entry: RawMaildirMbox) -> Result<Self, Self::Error> {
        let subdir_name = mail_entry.path().file_name();
        Ok(Self {
            name: subdir_name
                .and_then(OsStr::to_str)
                .and_then(|s| if s.len() < 2 { None } else { Some(&s[1..]) })
                .ok_or_else(|| {
                    anyhow!(
                        "cannot parse maildir subdirectory name from path {:?}",
                        subdir_name,
                    )
                })?
                .into(),
        })
    }
}
