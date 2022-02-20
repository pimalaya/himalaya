//! IMAP envelope module.
//!
//! This module provides IMAP types and conversion utilities related
//! to the envelope.

use anyhow::{anyhow, Context, Error, Result};
use std::{convert::TryFrom, ops::Deref};

use crate::{
    domain::{msg::Flag, Flags},
    msg::PrintableEnvelopes,
    output::{PrintTable, PrintTableOpts, WriteColor},
    ui::{Cell, Row, Table},
};

/// Represents a list of IMAP envelopes.
#[derive(Debug, Default, serde::Serialize)]
pub struct ImapEnvelopes(pub Vec<ImapEnvelope>);

impl Deref for ImapEnvelopes {
    type Target = Vec<ImapEnvelope>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PrintTable for ImapEnvelopes {
    fn print_table(&self, writter: &mut dyn WriteColor, opts: PrintTableOpts) -> Result<()> {
        writeln!(writter)?;
        Table::print(writter, self, opts)?;
        writeln!(writter)?;
        Ok(())
    }
}

impl PrintableEnvelopes for ImapEnvelopes {
    //
}

/// Represents the IMAP envelope. The envelope is just a message
/// subset, and is mostly used for listings.
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct ImapEnvelope {
    /// Represents the sequence number of the message.
    ///
    /// [RFC3501]: https://datatracker.ietf.org/doc/html/rfc3501#section-2.3.1.2
    pub id: u32,

    /// Represents the flags attached to the message.
    pub flags: Flags,

    /// Represents the subject of the message.
    pub subject: String,

    /// Represents the first sender of the message.
    pub sender: String,

    /// Represents the internal date of the message.
    ///
    /// [RFC3501]: https://datatracker.ietf.org/doc/html/rfc3501#section-2.3.3
    pub date: Option<String>,
}

impl Table for ImapEnvelope {
    fn head() -> Row {
        Row::new()
            .cell(Cell::new("ID").bold().underline().white())
            .cell(Cell::new("FLAGS").bold().underline().white())
            .cell(Cell::new("SUBJECT").shrinkable().bold().underline().white())
            .cell(Cell::new("SENDER").bold().underline().white())
            .cell(Cell::new("DATE").bold().underline().white())
    }

    fn row(&self) -> Row {
        let id = self.id.to_string();
        let flags = self.flags.to_symbols_string();
        let unseen = !self.flags.contains(&Flag::Seen);
        let subject = &self.subject;
        let sender = &self.sender;
        let date = self.date.as_deref().unwrap_or_default();
        Row::new()
            .cell(Cell::new(id).bold_if(unseen).red())
            .cell(Cell::new(flags).bold_if(unseen).white())
            .cell(Cell::new(subject).shrinkable().bold_if(unseen).green())
            .cell(Cell::new(sender).bold_if(unseen).blue())
            .cell(Cell::new(date).bold_if(unseen).yellow())
    }
}

/// Represents a list of raw envelopes returned by the `imap` crate.
pub type RawImapEnvelopes = imap::types::ZeroCopy<Vec<RawImapEnvelope>>;

impl TryFrom<RawImapEnvelopes> for ImapEnvelopes {
    type Error = Error;

    fn try_from(raw_envelopes: RawImapEnvelopes) -> Result<Self, Self::Error> {
        let mut envelopes = vec![];
        for raw_envelope in raw_envelopes.iter().rev() {
            envelopes.push(ImapEnvelope::try_from(raw_envelope).context("cannot parse envelope")?);
        }
        Ok(Self(envelopes))
    }
}

/// Represents the raw envelope returned by the `imap` crate.
pub type RawImapEnvelope = imap::types::Fetch;

impl TryFrom<&RawImapEnvelope> for ImapEnvelope {
    type Error = Error;

    fn try_from(fetch: &RawImapEnvelope) -> Result<ImapEnvelope> {
        let envelope = fetch
            .envelope()
            .ok_or_else(|| anyhow!("cannot get envelope of message {}", fetch.message))?;

        // Get the sequence number
        let id = fetch.message;

        // Get the flags
        let flags = Flags::try_from(fetch.flags())?;

        // Get the subject
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
