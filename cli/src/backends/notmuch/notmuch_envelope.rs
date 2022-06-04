//! Notmuch mailbox module.
//!
//! This module provides Notmuch types and conversion utilities
//! related to the envelope

use anyhow::{anyhow, Context, Result};
use chrono::DateTime;
use himalaya_lib::msg::{Envelope, Flag};
use log::{info, trace};

use crate::msg::{from_slice_to_addrs, Addr};

/// Represents the raw envelope returned by the `notmuch` crate.
pub type RawNotmuchEnvelope = notmuch::Message;

pub fn from_notmuch_msg(raw_envelope: RawNotmuchEnvelope) -> Result<Envelope> {
    info!("begin: try building envelope from notmuch parsed mail");

    let internal_id = raw_envelope.id().to_string();
    let id = format!("{:x}", md5::compute(&internal_id));
    let subject = raw_envelope
        .header("subject")
        .context("cannot get header \"Subject\" from notmuch message")?
        .unwrap_or_default()
        .to_string();
    let sender = raw_envelope
        .header("from")
        .context("cannot get header \"From\" from notmuch message")?
        .ok_or_else(|| anyhow!("cannot parse sender from notmuch message {:?}", internal_id))?
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
        .ok_or_else(|| anyhow!("cannot parse date of notmuch message {:?}", internal_id))?
        .to_string();
    let date = DateTime::parse_from_rfc2822(date.split_at(date.find(" (").unwrap_or(date.len())).0)
        .context(format!(
            "cannot parse message date {:?} of notmuch message {:?}",
            date, internal_id
        ))
        .map(|date| date.naive_local().to_string())
        .ok();

    let envelope = Envelope {
        id,
        internal_id,
        flags: raw_envelope
            .tags()
            .map(|tag| Flag::Custom(tag.to_string()))
            .collect(),
        subject,
        sender,
        date,
    };
    trace!("envelope: {:?}", envelope);

    info!("end: try building envelope from notmuch parsed mail");
    Ok(envelope)
}
