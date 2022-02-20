//! Maildir mailbox module.
//!
//! This module provides Maildir types and conversion utilities
//! related to the envelope

use anyhow::{anyhow, Context, Error, Result};
use log::{debug, info, trace};
use std::{
    convert::{TryFrom, TryInto},
    ops::Deref,
};

use crate::{
    backends::{MaildirFlag, MaildirFlags},
    domain::from_slice_to_addrs,
    msg::Envelopes,
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

impl PrintTable for MaildirEnvelopes {
    fn print_table(&self, writter: &mut dyn WriteColor, opts: PrintTableOpts) -> Result<()> {
        writeln!(writter)?;
        Table::print(writter, self, opts)?;
        writeln!(writter)?;
        Ok(())
    }
}

impl Envelopes for MaildirEnvelopes {
    //
}

/// Represents the envelope. The envelope is just a message subset,
/// and is mostly used for listings.
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct MaildirEnvelope {
    /// Represents the id of the message.
    pub id: String,

    /// Represents the flags of the message.
    pub flags: MaildirFlags,

    /// Represents the subject of the message.
    pub subject: String,

    /// Represents the first sender of the message.
    pub sender: String,
}

impl Table for MaildirEnvelope {
    fn head() -> Row {
        Row::new()
            .cell(Cell::new("ID").bold().underline().white())
            .cell(Cell::new("FLAGS").bold().underline().white())
            .cell(Cell::new("SUBJECT").shrinkable().bold().underline().white())
            .cell(Cell::new("SENDER").bold().underline().white())
    }

    fn row(&self) -> Row {
        let id = self.id.to_string();
        let unseen = !self.flags.contains(&MaildirFlag::Seen);
        let flags = self.flags.to_symbols_string();
        let subject = &self.subject;
        let sender = &self.sender;
        Row::new()
            .cell(Cell::new(id).bold_if(unseen).red())
            .cell(Cell::new(flags).bold_if(unseen).white())
            .cell(Cell::new(subject).shrinkable().bold_if(unseen).green())
            .cell(Cell::new(sender).bold_if(unseen).blue())
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
        info!("begin: build envelope from maildir parsed mail");

        let mut envelope = Self::default();
        envelope.flags = (&mail_entry)
            .try_into()
            .context("cannot parse maildir flags")?;

        let parsed_mail = mail_entry
            .parsed()
            .context("cannot parse maildir mail entry")?;
        trace!("parsed mail: {:?}", parsed_mail);

        debug!("parsing headers");
        for header in parsed_mail.get_headers() {
            let key = header.get_key();
            debug!("header key: {:?}", key);

            let val = header.get_value();
            let val = String::from_utf8(header.get_value_raw().to_vec())
                .map(|val| val.trim().to_string())
                .context(format!(
                    "cannot decode value {:?} from header {:?}",
                    key, val
                ))?;
            debug!("header value: {:?}", val);

            match key.to_lowercase().as_str() {
                "subject" => {
                    envelope.subject = val.into();
                }
                "from" => {
                    envelope.sender = from_slice_to_addrs(val)
                        .context(format!("cannot parse header {:?}", key))?
                        .ok_or_else(|| anyhow!("cannot find sender"))?
                        .to_string()
                }
                _ => (),
            }
        }

        envelope.id = mail_entry.id().into();

        trace!("envelope: {:?}", envelope);
        info!("end: building envelope from parsed mail");
        Ok(envelope)
    }
}
