//! IMAP envelope module.
//!
//! This module provides IMAP types and conversion utilities related
//! to the envelope.

use anyhow::{anyhow, Context, Result};
use himalaya_lib::msg::Envelope;

use super::from_imap_flags;

/// Represents the raw envelope returned by the `imap` crate.
pub type ImapFetch = imap::types::Fetch;

pub fn from_imap_fetch(fetch: &ImapFetch) -> Result<Envelope> {
    let envelope = fetch
        .envelope()
        .ok_or_else(|| anyhow!("cannot get envelope of message {}", fetch.message))?;

    let id = fetch.message.to_string();

    let flags = from_imap_flags(fetch.flags());

    let subject = envelope
        .subject
        .as_ref()
        .map(|subj| {
            rfc2047_decoder::decode(subj).context(format!(
                "cannot decode subject of message {}",
                fetch.message
            ))
        })
        .unwrap_or_else(|| Ok(String::default()))?;

    let sender = envelope
        .sender
        .as_ref()
        .and_then(|addrs| addrs.get(0))
        .or_else(|| envelope.from.as_ref().and_then(|addrs| addrs.get(0)))
        .ok_or_else(|| anyhow!("cannot get sender of message {}", fetch.message))?;
    let sender = if let Some(ref name) = sender.name {
        rfc2047_decoder::decode(&name.to_vec()).context(format!(
            "cannot decode sender's name of message {}",
            fetch.message,
        ))?
    } else {
        let mbox = sender
            .mailbox
            .as_ref()
            .ok_or_else(|| anyhow!("cannot get sender's mailbox of message {}", fetch.message))
            .and_then(|mbox| {
                rfc2047_decoder::decode(&mbox.to_vec()).context(format!(
                    "cannot decode sender's mailbox of message {}",
                    fetch.message,
                ))
            })?;
        let host = sender
            .host
            .as_ref()
            .ok_or_else(|| anyhow!("cannot get sender's host of message {}", fetch.message))
            .and_then(|host| {
                rfc2047_decoder::decode(&host.to_vec()).context(format!(
                    "cannot decode sender's host of message {}",
                    fetch.message,
                ))
            })?;
        format!("{}@{}", mbox, host)
    };

    let date = fetch
        .internal_date()
        .map(|date| date.naive_local().to_string());

    Ok(Envelope {
        id: id.clone(),
        internal_id: id,
        flags,
        subject,
        sender,
        date,
    })
}
