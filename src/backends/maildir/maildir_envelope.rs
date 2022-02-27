//! Maildir mailbox module.
//!
//! This module provides Maildir types and conversion utilities
//! related to the envelope

use anyhow::{anyhow, Context, Error, Result};
use chrono::DateTime;
use log::{debug, info, trace};
use std::{
    convert::{TryFrom, TryInto},
    ops::{Deref, DerefMut},
};

use crate::{
    backends::{MaildirFlag, MaildirFlags},
    msg::{from_slice_to_addrs, Addr},
    output::{PrintTable, PrintTableOpts, WriteColor},
    ui::{Cell, Row, Table},
};

/// Represents a list of envelopes.
#[derive(Debug, Default, serde::Serialize)]
pub struct MaildirEnvelopes(pub Vec<MaildirEnvelope>);

impl Deref for MaildirEnvelopes {
    type Target = Vec<MaildirEnvelope>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MaildirEnvelopes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl PrintTable for MaildirEnvelopes {
    fn print_table(&self, writter: &mut dyn WriteColor, opts: PrintTableOpts) -> Result<()> {
        writeln!(writter)?;
        Table::print(writter, self, opts)?;
        writeln!(writter)?;
        Ok(())
    }
}

// impl Envelopes for MaildirEnvelopes {
//     //
// }

/// Represents the envelope. The envelope is just a message subset,
/// and is mostly used for listings.
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct MaildirEnvelope {
    /// Represents the id of the message.
    pub id: String,

    /// Represents the MD5 hash of the message id.
    pub hash: String,

    /// Represents the flags of the message.
    pub flags: MaildirFlags,

    /// Represents the subject of the message.
    pub subject: String,

    /// Represents the first sender of the message.
    pub sender: String,

    /// Represents the date of the message.
    pub date: String,
}

impl Table for MaildirEnvelope {
    fn head() -> Row {
        Row::new()
            .cell(Cell::new("HASH").bold().underline().white())
            .cell(Cell::new("FLAGS").bold().underline().white())
            .cell(Cell::new("SUBJECT").shrinkable().bold().underline().white())
            .cell(Cell::new("SENDER").bold().underline().white())
            .cell(Cell::new("DATE").bold().underline().white())
    }

    fn row(&self) -> Row {
        let hash = self.hash.clone();
        let unseen = !self.flags.contains(&MaildirFlag::Seen);
        let flags = self.flags.to_symbols_string();
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

/// Represents a list of raw envelopees returned by the `maildir` crate.
pub type RawMaildirEnvelopes = maildir::MailEntries;

impl<'a> TryFrom<RawMaildirEnvelopes> for MaildirEnvelopes {
    type Error = Error;

    fn try_from(mail_entries: RawMaildirEnvelopes) -> Result<Self, Self::Error> {
        let mut envelopes = vec![];
        for entry in mail_entries {
            let envelope: MaildirEnvelope = entry
                .context("cannot decode maildir mail entry")?
                .try_into()
                .context("cannot parse maildir mail entry")?;
            envelopes.push(envelope);
        }

        Ok(MaildirEnvelopes(envelopes))
    }
}

/// Represents the raw envelope returned by the `maildir` crate.
pub type RawMaildirEnvelope = maildir::MailEntry;

impl<'a> TryFrom<RawMaildirEnvelope> for MaildirEnvelope {
    type Error = Error;

    fn try_from(mut mail_entry: RawMaildirEnvelope) -> Result<Self, Self::Error> {
        info!("begin: try building envelope from maildir parsed mail");

        let mut envelope = Self::default();

        envelope.id = mail_entry.id().into();
        envelope.hash = format!("{:x}", md5::compute(&envelope.id));
        envelope.flags = (&mail_entry)
            .try_into()
            .context("cannot parse maildir flags")?;

        let parsed_mail = mail_entry
            .parsed()
            .context("cannot parse maildir mail entry")?;

        debug!("begin: parse headers");
        for h in parsed_mail.get_headers() {
            let k = h.get_key();
            debug!("header key: {:?}", k);

            let v = rfc2047_decoder::decode(h.get_value_raw())
                .context(format!("cannot decode value from header {:?}", k))?;
            debug!("header value: {:?}", v);

            match k.to_lowercase().as_str() {
                "date" => {
                    envelope.date =
                        DateTime::parse_from_rfc2822(v.split_at(v.find(" (").unwrap_or(v.len())).0)
                            .context(format!("cannot parse maildir message date {:?}", v))?
                            .naive_local()
                            .to_string();
                }
                "subject" => {
                    envelope.subject = v.into();
                }
                "from" => {
                    envelope.sender = from_slice_to_addrs(v)
                        .context(format!("cannot parse header {:?}", k))?
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
                            Addr::Group(mailparse::GroupInfo { group_name, .. }) => {
                                group_name.to_owned()
                            }
                        })
                        .ok_or_else(|| anyhow!("cannot find sender"))?;
                }
                _ => (),
            }
        }
        debug!("end: parse headers");

        trace!("envelope: {:?}", envelope);
        info!("end: try building envelope from maildir parsed mail");
        Ok(envelope)
    }
}
