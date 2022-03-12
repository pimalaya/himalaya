//! Notmuch mailbox module.
//!
//! This module provides Notmuch types and conversion utilities
//! related to the envelope

use anyhow::{anyhow, Context, Error, Result};
use chrono::DateTime;
use log::{info, trace};
use std::{
    convert::{TryFrom, TryInto},
    ops::{Deref, DerefMut},
};

use crate::{
    msg::{from_slice_to_addrs, Addr},
    output::{PrintTable, PrintTableOpts, WriteColor},
    ui::{Cell, Row, Table},
};

/// Represents a list of envelopes.
#[derive(Debug, Default, serde::Serialize)]
pub struct NotmuchEnvelopes {
    #[serde(rename = "response")]
    pub envelopes: Vec<NotmuchEnvelope>,
}

impl Deref for NotmuchEnvelopes {
    type Target = Vec<NotmuchEnvelope>;

    fn deref(&self) -> &Self::Target {
        &self.envelopes
    }
}

impl DerefMut for NotmuchEnvelopes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.envelopes
    }
}

impl PrintTable for NotmuchEnvelopes {
    fn print_table(&self, writer: &mut dyn WriteColor, opts: PrintTableOpts) -> Result<()> {
        writeln!(writer)?;
        Table::print(writer, self, opts)?;
        writeln!(writer)?;
        Ok(())
    }
}

/// Represents the envelope. The envelope is just a message subset,
/// and is mostly used for listings.
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct NotmuchEnvelope {
    /// Represents the id of the message.
    pub id: String,

    /// Represents the MD5 hash of the message id.
    pub hash: String,

    /// Represents the tags of the message.
    pub flags: Vec<String>,

    /// Represents the subject of the message.
    pub subject: String,

    /// Represents the first sender of the message.
    pub sender: String,

    /// Represents the date of the message.
    pub date: String,
}

impl Table for NotmuchEnvelope {
    fn head() -> Row {
        Row::new()
            .cell(Cell::new("HASH").bold().underline().white())
            .cell(Cell::new("FLAGS").bold().underline().white())
            .cell(Cell::new("SUBJECT").shrinkable().bold().underline().white())
            .cell(Cell::new("SENDER").bold().underline().white())
            .cell(Cell::new("DATE").bold().underline().white())
    }

    fn row(&self) -> Row {
        let hash = self.hash.to_string();
        let unseen = !self.flags.contains(&String::from("unread"));
        let flags = String::new();
        let subject = &self.subject;
        let sender = &self.sender;
        let date = &self.date;
        Row::new()
            .cell(Cell::new(hash).bold_if(unseen).red())
            .cell(Cell::new(flags).bold_if(unseen).white())
            .cell(Cell::new(subject).shrinkable().bold_if(unseen).green())
            .cell(Cell::new(sender).bold_if(unseen).blue())
            .cell(Cell::new(date).bold_if(unseen).yellow())
    }
}

/// Represents a list of raw envelopees returned by the `notmuch` crate.
pub type RawNotmuchEnvelopes = notmuch::Messages;

impl<'a> TryFrom<RawNotmuchEnvelopes> for NotmuchEnvelopes {
    type Error = Error;

    fn try_from(raw_envelopes: RawNotmuchEnvelopes) -> Result<Self, Self::Error> {
        let mut envelopes = vec![];
        for raw_envelope in raw_envelopes {
            let envelope: NotmuchEnvelope = raw_envelope
                .try_into()
                .context("cannot parse notmuch mail entry")?;
            envelopes.push(envelope);
        }
        Ok(NotmuchEnvelopes { envelopes })
    }
}

/// Represents the raw envelope returned by the `notmuch` crate.
pub type RawNotmuchEnvelope = notmuch::Message;

impl<'a> TryFrom<RawNotmuchEnvelope> for NotmuchEnvelope {
    type Error = Error;

    fn try_from(raw_envelope: RawNotmuchEnvelope) -> Result<Self, Self::Error> {
        info!("begin: try building envelope from notmuch parsed mail");

        let id = raw_envelope.id().to_string();
        let hash = format!("{:x}", md5::compute(&id));
        let subject = raw_envelope
            .header("subject")
            .context("cannot get header \"Subject\" from notmuch message")?
            .unwrap_or_default()
            .to_string();
        let sender = raw_envelope
            .header("from")
            .context("cannot get header \"From\" from notmuch message")?
            .ok_or_else(|| anyhow!("cannot parse sender from notmuch message {:?}", id))?
            .to_string();
        let sender = from_slice_to_addrs(sender)?
            .and_then(|senders| {
                if senders.is_empty() {
                    None
                } else {
                    Some(senders)
                }
            })
            .map(|senders| match &senders[0] {
                Addr::Single(mailparse::SingleInfo { display_name, addr }) => {
                    display_name.as_ref().unwrap_or_else(|| addr).to_owned()
                }
                Addr::Group(mailparse::GroupInfo { group_name, .. }) => group_name.to_owned(),
            })
            .ok_or_else(|| anyhow!("cannot find sender"))?;
        let date = raw_envelope
            .header("date")
            .context("cannot get header \"Date\" from notmuch message")?
            .ok_or_else(|| anyhow!("cannot parse date of notmuch message {:?}", id))?
            .to_string();
        let date =
            DateTime::parse_from_rfc2822(date.split_at(date.find(" (").unwrap_or(date.len())).0)
                .context(format!(
                    "cannot parse message date {:?} of notmuch message {:?}",
                    date, id
                ))?
                .naive_local()
                .to_string();

        let envelope = Self {
            id,
            hash,
            flags: raw_envelope.tags().collect(),
            subject,
            sender,
            date,
        };
        trace!("envelope: {:?}", envelope);

        info!("end: try building envelope from notmuch parsed mail");
        Ok(envelope)
    }
}
