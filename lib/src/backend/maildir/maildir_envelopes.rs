//! Maildir mailbox module.
//!
//! This module provides Maildir types and conversion utilities
//! related to the envelope.

use crate::{backend::backend::Result, msg::Envelopes};

use super::{maildir_envelope, MaildirError};

/// Represents a list of raw envelopees returned by the `maildir`
/// crate.
pub type MaildirEnvelopes = maildir::MailEntries;

pub fn from_maildir_entries(mail_entries: MaildirEnvelopes) -> Result<Envelopes> {
    let mut envelopes = Envelopes::default();
    for entry in mail_entries {
        let entry = entry.map_err(MaildirError::DecodeEntryError)?;
        envelopes.push(maildir_envelope::from_maildir_entry(entry)?);
    }
    Ok(envelopes)
}
