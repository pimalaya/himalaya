//! # Listing
//!
//! What Himalaya's lists print beyond pimalaya-cli's [`Listing`] frame.
//!
//! [`Listing`]: pimalaya_cli::listing::Listing

/// The unread and total counts of a mailbox as one cell, `3/120`, a count
/// the backend gave none of as `-`, nothing when it gave neither.
#[cfg(any(backend, feature = "smtp"))]
pub fn counts(unread: Option<u64>, total: Option<u64>) -> String {
    let count = |n: Option<u64>| n.map_or_else(|| String::from("-"), |n| n.to_string());

    match (unread, total) {
        (None, None) => String::new(),
        (unread, total) => format!("{}/{}", count(unread), count(total)),
    }
}
