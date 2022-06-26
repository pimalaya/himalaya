use anyhow::{Context, Result};
use himalaya_lib::{
    backend::{from_imap_fetch, ImapFetch},
    msg::Envelopes,
};

/// Represents the list of raw envelopes returned by the `imap` crate.
pub type ImapFetches = imap::types::ZeroCopy<Vec<ImapFetch>>;

pub fn from_imap_fetches(fetches: ImapFetches) -> Result<Envelopes> {
    let mut envelopes = Envelopes::default();
    for fetch in fetches.iter().rev() {
        envelopes.push(from_imap_fetch(fetch).context("cannot parse imap fetch")?);
    }
    Ok(envelopes)
}
