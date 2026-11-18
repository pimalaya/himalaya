//! # Address
//!
//! An email address shared across all protocols.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A single email address with an optional display name.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Address {
    /// The display name, `Alice` in `Alice <alice@example.org>`.
    pub name: Option<String>,
    /// The address itself.
    pub email: String,
}

/// The mailboxes of a mime-rs address list, groups flattened, those with
/// no address left out.
#[cfg(any(
    feature = "gmail",
    feature = "maildir",
    feature = "mbox",
    feature = "pimdir"
))]
pub fn from_mime(list: &mime::value::address::MimeAddressList) -> Vec<Address> {
    list.mailboxes()
        .filter_map(|mailbox| {
            let email = mailbox.address.clone().filter(|email| !email.is_empty())?;
            Some(Address {
                name: mailbox.name.clone(),
                email,
            })
        })
        .collect()
}

/// The mailboxes of an address header's value, read by mime-rs as
/// [`from_mime`] reads a parsed one.
#[cfg(feature = "gmail")]
pub fn parse(value: &[u8]) -> Vec<Address> {
    from_mime(&mime::tree::field::address::parse_addresses(value))
}
