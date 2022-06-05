//! Maildir mailbox module.
//!
//! This module provides Maildir types and conversion utilities
//! related to the envelope

use himalaya_lib::msg::Envelopes;
use anyhow::{Result, Context};

use super::maildir_envelope;

/// Represents a list of raw envelopees returned by the `maildir` crate.
pub type MaildirEnvelopes = maildir::MailEntries;

pub fn from_maildir_entries(mail_entries: MaildirEnvelopes) -> Result<Envelopes> {
    let mut envelopes = Envelopes::default();
    for entry in mail_entries {
        envelopes.push(
            maildir_envelope::from_maildir_entry(
                entry.context("cannot decode maildir mail entry")?,
            )
            .context("cannot parse maildir mail entry")?,
        );
    }
    Ok(envelopes)
}
