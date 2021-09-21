use anyhow::{anyhow, Context, Error, Result};
use imap::types::{Fetch, Flag};
use lettre::Address;
use serde::Serialize;
use std::convert::TryFrom;

use crate::{
    domain::msg::Flags,
    ui::table::{Cell, Row, Table},
};

type Sender = lettre::message::Mailbox;

/// Representation of an envelope. An envelope gathers basic information related to a message. It
/// is mostly used for listings.
#[derive(Debug, Serialize)]
pub struct Envelope {
    /// The sequence number of the message.
    ///
    /// [RFC3501]: https://datatracker.ietf.org/doc/html/rfc3501#section-2.3.1.2
    pub id: u32,

    /// The flags attached to the message.
    pub flags: Flags,

    /// The subject of the message.
    pub subject: String,

    /// The sender of the message.
    pub sender: Sender,

    /// The internal date of the message.
    ///
    /// [RFC3501]: https://datatracker.ietf.org/doc/html/rfc3501#section-2.3.3
    pub date: Option<String>,
}

impl Default for Envelope {
    fn default() -> Self {
        Self {
            id: Default::default(),
            flags: Default::default(),
            subject: Default::default(),
            sender: Sender::new(
                Default::default(),
                Address::new("user", "localhost").unwrap(),
            ),
            date: Default::default(),
        }
    }
}

impl<'a> TryFrom<&'a Fetch> for Envelope {
    type Error = Error;

    fn try_from(fetch: &'a Fetch) -> Result<Envelope> {
        let envelope = fetch
            .envelope()
            .ok_or(anyhow!("cannot get envelope of message {}", fetch.message))?;

        // Get the sequence number
        let id = fetch.message;

        // Get the flags
        let flags = Flags::try_from(fetch.flags())?;

        // Get the subject
        let subject = envelope
            .subject
            .as_ref()
            .ok_or(anyhow!("cannot get subject of message {}", fetch.message))
            .and_then(|subj| {
                rfc2047_decoder::decode(subj).context(format!(
                    "cannot decode subject of message {}",
                    fetch.message
                ))
            })?;

        // Get the sender
        let sender = envelope
            .sender
            .as_ref()
            .and_then(|addrs| addrs.get(0))
            .or_else(|| envelope.from.as_ref().and_then(|addrs| addrs.get(0)))
            .ok_or(anyhow!("cannot get sender of message {}", fetch.message))?;
        let sender_name = sender
            .name
            .as_ref()
            .map(|name| {
                rfc2047_decoder::decode(&name.to_vec())
                    .context(format!(
                        "cannot decode sender's name of message {}",
                        fetch.message,
                    ))
                    .map(|name| Some(name))
            })
            .unwrap_or(Ok(None))?;
        let sender_mbox = sender
            .mailbox
            .as_ref()
            .ok_or(anyhow!(
                "cannot get sender's mailbox of message {}",
                fetch.message
            ))
            .and_then(|mbox| {
                rfc2047_decoder::decode(&mbox.to_vec()).context(format!(
                    "cannot decode sender's mailbox of message {}",
                    fetch.message,
                ))
            })?;
        let sender_host = sender
            .host
            .as_ref()
            .ok_or(anyhow!(
                "cannot get sender's host of message {}",
                fetch.message
            ))
            .and_then(|host| {
                rfc2047_decoder::decode(&host.to_vec()).context(format!(
                    "cannot decode sender's host of message {}",
                    fetch.message,
                ))
            })?;
        let sender = Sender::new(sender_name, Address::new(sender_mbox, sender_host)?);

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

impl Table for Envelope {
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
        let sender = self
            .sender
            .name
            .to_owned()
            .unwrap_or_else(|| self.sender.email.to_string());
        let date = self
            .date
            .as_ref()
            .map(|date| date.as_str())
            .unwrap_or_default();

        Row::new()
            .cell(Cell::new(id).bold_if(unseen).red())
            .cell(Cell::new(flags).bold_if(unseen).white())
            .cell(Cell::new(subject).shrinkable().bold_if(unseen).green())
            .cell(Cell::new(sender.to_string()).bold_if(unseen).blue())
            .cell(Cell::new(date).bold_if(unseen).yellow())
    }
}
