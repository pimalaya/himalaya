use ammonia;
use anyhow::{anyhow, Context, Error, Result};
use htmlescape;
use regex::Regex;
use serde::Serialize;
use std::convert::TryFrom;

use crate::msg::{Flags, Parts};

type Addr = lettre::message::Mailbox;

/// Representation of a message.
#[derive(Debug, Default, Serialize)]
pub struct Msg {
    /// The sequence number of the message.
    ///
    /// [RFC3501]: https://datatracker.ietf.org/doc/html/rfc3501#section-2.3.1.2
    pub id: u32,

    /// The flags attached to the message.
    pub flags: Flags,

    /// The subject of the message.
    pub subject: String,

    pub from: Option<Vec<Addr>>,
    pub reply_to: Option<Vec<Addr>>,
    pub to: Option<Vec<Addr>>,
    pub cc: Option<Vec<Addr>>,
    pub bcc: Option<Vec<Addr>>,
    pub in_reply_to: Option<String>,
    pub message_id: Option<String>,

    /// The internal date of the message.
    ///
    /// [RFC3501]: https://datatracker.ietf.org/doc/html/rfc3501#section-2.3.3
    pub date: Option<String>,
    pub parts: Parts,
}

impl Msg {
    pub fn join_text_parts(&self) -> String {
        let rg = Regex::new(r"(\r?\n){2,}").unwrap();
        self.parts
            .get("text/plain")
            .map(|parts| parts.join("\n\n"))
            .map(|text| {
                ammonia::Builder::new()
                    .tags(Default::default())
                    .clean(&text)
                    .to_string()
            })
            .map(|text| match htmlescape::decode_html(&text) {
                Ok(text) => text,
                Err(_) => text,
            })
            .or_else(|| self.parts.get("text/html").map(|parts| parts.join("\n\n")))
            .map(|text| rg.replace_all(&text.trim(), "\n\n").to_string())
            .unwrap_or_default()
    }
}

impl<'a> TryFrom<&'a imap::types::Fetch> for Msg {
    type Error = Error;

    fn try_from(fetch: &'a imap::types::Fetch) -> Result<Msg> {
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

        // Get the sender(s) address(es)
        let from = match envelope
            .sender
            .as_ref()
            .or_else(|| envelope.from.as_ref())
            .map(parse_addrs)
        {
            Some(addrs) => Some(addrs?),
            None => None,
        };

        // Get the "Reply-To" address(es)
        let reply_to = parse_some_addrs(&envelope.reply_to).context(format!(
            r#"cannot parse "reply to" address of message {}"#,
            id
        ))?;

        // Get the recipient(s) address(es)
        let to = parse_some_addrs(&envelope.to)
            .context(format!(r#"cannot parse "to" address of message {}"#, id))?;

        // Get the "Cc" recipient(s) address(es)
        let cc = parse_some_addrs(&envelope.cc)
            .context(format!(r#"cannot parse "cc" address of message {}"#, id))?;

        // Get the "Bcc" recipient(s) address(es)
        let bcc = parse_some_addrs(&envelope.bcc)
            .context(format!(r#"cannot parse "bcc" address of message {}"#, id))?;

        // Get the "In-Reply-To" message identifier
        let in_reply_to = match envelope
            .in_reply_to
            .as_ref()
            .map(|cow| String::from_utf8(cow.to_vec()))
        {
            Some(id) => Some(id?),
            None => None,
        };

        // Get the message identifier
        let message_id = match envelope
            .message_id
            .as_ref()
            .map(|cow| String::from_utf8(cow.to_vec()))
        {
            Some(id) => Some(id?),
            None => None,
        };

        // Get the internal date
        let date = fetch
            .internal_date()
            .map(|date| date.format("%Y/%m/%d %H:%M").to_string());
        let parts = Parts::from(
            &mailparse::parse_mail(
                fetch
                    .body()
                    .ok_or(anyhow!("cannot get body of message {}", id))?,
            )
            .context(format!("cannot parse body of message {}", id))?,
        );

        Ok(Self {
            id,
            flags,
            subject,
            from,
            reply_to,
            to,
            cc,
            bcc,
            in_reply_to,
            message_id,
            date,
            parts,
        })
    }
}

pub fn parse_addr(addr: &imap_proto::Address) -> Result<Addr> {
    let name = addr
        .name
        .as_ref()
        .map(|name| {
            rfc2047_decoder::decode(&name.to_vec())
                .context("cannot decode address name")
                .map(|name| Some(name))
        })
        .unwrap_or(Ok(None))?;
    let mbox = addr
        .mailbox
        .as_ref()
        .ok_or(anyhow!("cannot get address mailbox"))
        .and_then(|mbox| {
            rfc2047_decoder::decode(&mbox.to_vec()).context("cannot decode address mailbox")
        })?;
    let host = addr
        .host
        .as_ref()
        .ok_or(anyhow!("cannot get address host"))
        .and_then(|host| {
            rfc2047_decoder::decode(&host.to_vec()).context("cannot decode address host")
        })?;

    Ok(Addr::new(name, lettre::Address::new(mbox, host)?))
}

pub fn parse_addrs(addrs: &Vec<imap_proto::Address>) -> Result<Vec<Addr>> {
    let mut parsed_addrs = vec![];
    for addr in addrs {
        parsed_addrs
            .push(parse_addr(addr).context(format!(r#"cannot parse address "{:?}""#, addr))?);
    }
    Ok(parsed_addrs)
}

pub fn parse_some_addrs(addrs: &Option<Vec<imap_proto::Address>>) -> Result<Option<Vec<Addr>>> {
    Ok(match addrs.as_ref().map(parse_addrs) {
        Some(addrs) => Some(addrs?),
        None => None,
    })
}
