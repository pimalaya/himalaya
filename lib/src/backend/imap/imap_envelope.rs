//! IMAP envelope module.
//!
//! This module provides IMAP types and conversion utilities related
//! to the envelope.

use rfc2047_decoder;

use crate::{
    backend::{
        from_imap_flags,
        imap::{Error, Result},
    },
    msg::Envelope,
};

/// Represents the raw envelope returned by the `imap` crate.
pub type ImapFetch = imap::types::Fetch;

pub fn from_imap_fetch(fetch: &ImapFetch) -> Result<Envelope> {
    let envelope = fetch
        .envelope()
        .ok_or_else(|| Error::GetEnvelopeError(fetch.message))?;

    let id = fetch.message.to_string();

    let flags = from_imap_flags(fetch.flags());

    let subject = envelope
        .subject
        .as_ref()
        .map(|subj| {
            rfc2047_decoder::decode(subj)
                .map_err(|err| Error::DecodeSubjectError(err, fetch.message))
        })
        .unwrap_or_else(|| Ok(String::default()))?;

    let sender = envelope
        .sender
        .as_ref()
        .and_then(|addrs| addrs.get(0))
        .or_else(|| envelope.from.as_ref().and_then(|addrs| addrs.get(0)))
        .ok_or_else(|| Error::GetSenderError(fetch.message))?;
    let sender = if let Some(ref name) = sender.name {
        rfc2047_decoder::decode(&name.to_vec())
            .map_err(|err| Error::DecodeSenderNameError(err, fetch.message))?
    } else {
        let mbox = sender
            .mailbox
            .as_ref()
            .ok_or_else(|| Error::GetSenderError(fetch.message))
            .and_then(|mbox| {
                rfc2047_decoder::decode(&mbox.to_vec())
                    .map_err(|err| Error::DecodeSenderNameError(err, fetch.message))
            })?;
        let host = sender
            .host
            .as_ref()
            .ok_or_else(|| Error::GetSenderError(fetch.message))
            .and_then(|host| {
                rfc2047_decoder::decode(&host.to_vec())
                    .map_err(|err| Error::DecodeSenderNameError(err, fetch.message))
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
