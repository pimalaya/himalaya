//! IMAP envelope module.
//!
//! This module provides IMAP types and conversion utilities related
//! to the envelope.

use anyhow::{anyhow, Context, Error, Result};
use std::{borrow::Cow, convert::TryFrom};

use crate::{
    domain::Flags,
    msg::{Envelope, Envelopes},
};

/// Represents a list of raw envelopes returned by the `imap` crate.
pub type RawImapEnvelopes = imap::types::ZeroCopy<Vec<RawImapEnvelope>>;

impl<'a> TryFrom<&'a RawImapEnvelopes> for Envelopes<'a> {
    type Error = Error;

    fn try_from(raw_envelopes: &'a RawImapEnvelopes) -> Result<Self, Self::Error> {
        let mut envelopes = vec![];
        for raw_envelope in raw_envelopes.iter().rev() {
            envelopes.push(Envelope::try_from(raw_envelope).context("cannot parse envelope")?);
        }
        Ok(Self(envelopes))
    }
}

/// Represents the raw envelope returned by the `imap` crate.
pub type RawImapEnvelope = imap::types::Fetch;

impl<'a> TryFrom<&'a RawImapEnvelope> for Envelope<'a> {
    type Error = Error;

    fn try_from(fetch: &'a RawImapEnvelope) -> Result<Envelope> {
        let envelope = fetch
            .envelope()
            .ok_or_else(|| anyhow!("cannot get envelope of message {}", fetch.message))?;

        // Get the sequence number
        let id = fetch.message.to_string().into();

        // Get the flags
        let flags = Flags::try_from(fetch.flags())?;

        // Get the subject
        let subject: Cow<str> = envelope
            .subject
            .as_ref()
            .map(|subj| {
                rfc2047_decoder::decode(subj).context(format!(
                    "cannot decode subject of message {}",
                    fetch.message
                ))
            })
            .unwrap_or_else(|| Ok(String::default()))?
            .into();

        // Get the sender
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

        // Get the internal date
        let date = fetch
            .internal_date()
            .map(|date| date.naive_local().to_string());

        Ok(Self {
            id,
            flags,
            subject,
            sender,
            date,
        })
    }
}
