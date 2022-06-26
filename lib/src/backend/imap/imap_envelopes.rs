use crate::{
    backend::{
        imap::{from_imap_fetch, Result},
        ImapFetch,
    },
    msg::Envelopes,
};

/// Represents the list of raw envelopes returned by the `imap` crate.
pub type ImapFetches = imap::types::ZeroCopy<Vec<ImapFetch>>;

pub fn from_imap_fetches(fetches: ImapFetches) -> Result<Envelopes> {
    let mut envelopes = Envelopes::default();
    for fetch in fetches.iter().rev() {
        envelopes.push(from_imap_fetch(fetch)?);
    }
    Ok(envelopes)
}
