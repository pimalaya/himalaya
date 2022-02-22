//! Module related to email addresses.
//!
//! This module regroups email address entities and converters.

use anyhow::{Context, Result};
use log::trace;
use mailparse;
use std::fmt::Debug;

/// Defines a single email address.
pub type Addr = mailparse::MailAddr;

/// Defines a list of email addresses.
pub type Addrs = mailparse::MailAddrList;

/// Converts a slice into an optional list of addresses.
pub fn from_slice_to_addrs<S: AsRef<str> + Debug>(addrs: S) -> Result<Option<Addrs>> {
    let addrs = mailparse::addrparse(addrs.as_ref())?;
    Ok(if addrs.is_empty() { None } else { Some(addrs) })
}

/// Converts a list of addresses into a list of [`lettre::message::Mailbox`].
pub fn from_addrs_to_sendable_mbox(addrs: &Addrs) -> Result<Vec<lettre::message::Mailbox>> {
    let mut sendable_addrs: Vec<lettre::message::Mailbox> = vec![];
    for addr in addrs.iter() {
        match addr {
            Addr::Single(mailparse::SingleInfo { display_name, addr }) => sendable_addrs.push(
                lettre::message::Mailbox::new(display_name.clone(), addr.parse()?),
            ),
            Addr::Group(mailparse::GroupInfo { group_name, addrs }) => {
                for addr in addrs {
                    sendable_addrs.push(lettre::message::Mailbox::new(
                        addr.display_name.clone().or(Some(group_name.clone())),
                        addr.to_string().parse()?,
                    ))
                }
            }
        }
    }
    Ok(sendable_addrs)
}

/// Converts a list of addresses into a list of [`lettre::Address`].
pub fn from_addrs_to_sendable_addrs(addrs: &Addrs) -> Result<Vec<lettre::Address>> {
    let mut sendable_addrs = vec![];
    for addr in addrs.iter() {
        match addr {
            mailparse::MailAddr::Single(mailparse::SingleInfo {
                display_name: _,
                addr,
            }) => {
                sendable_addrs.push(addr.parse()?);
            }
            mailparse::MailAddr::Group(mailparse::GroupInfo {
                group_name: _,
                addrs,
            }) => {
                for addr in addrs {
                    sendable_addrs.push(addr.addr.parse()?);
                }
            }
        };
    }
    Ok(sendable_addrs)
}

/// Converts a [`imap_proto::Address`] into an address.
pub fn from_imap_addr_to_addr(addr: &imap_proto::Address) -> Result<Addr> {
    let name = addr
        .name
        .as_ref()
        .map(|name| {
            rfc2047_decoder::decode(&name.to_vec())
                .context("cannot decode address name")
                .map(Some)
        })
        .unwrap_or(Ok(None))?;
    let mbox = addr
        .mailbox
        .as_ref()
        .map(|mbox| {
            rfc2047_decoder::decode(&mbox.to_vec())
                .context("cannot decode address mailbox")
                .map(Some)
        })
        .unwrap_or(Ok(None))?;
    let host = addr
        .host
        .as_ref()
        .map(|host| {
            rfc2047_decoder::decode(&host.to_vec())
                .context("cannot decode address host")
                .map(Some)
        })
        .unwrap_or(Ok(None))?;

    trace!("parsing address from imap address");
    trace!("name: {:?}", name);
    trace!("mbox: {:?}", mbox);
    trace!("host: {:?}", host);

    Ok(Addr::Single(mailparse::SingleInfo {
        display_name: name,
        addr: match host {
            Some(host) => format!("{}@{}", mbox.unwrap_or_default(), host),
            None => mbox.unwrap_or_default(),
        },
    }))
}

/// Converts a list of [`imap_proto::Address`] into a list of addresses.
pub fn from_imap_addrs_to_addrs(proto_addrs: &[imap_proto::Address]) -> Result<Addrs> {
    let mut addrs = vec![];
    for addr in proto_addrs {
        addrs.push(
            from_imap_addr_to_addr(addr).context(format!("cannot parse address {:?}", addr))?,
        );
    }
    Ok(addrs.into())
}

/// Converts an optional list of [`imap_proto::Address`] into an optional list of addresses.
pub fn from_imap_addrs_to_some_addrs(
    addrs: &Option<Vec<imap_proto::Address>>,
) -> Result<Option<Addrs>> {
    Ok(
        if let Some(addrs) = addrs.as_deref().map(from_imap_addrs_to_addrs) {
            Some(addrs?)
        } else {
            None
        },
    )
}
