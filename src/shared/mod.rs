#[cfg(any(feature = "imap", feature = "jmap", feature = "maildir"))]
pub mod attachments;
pub mod client;
pub mod envelopes;
pub mod flags;
pub mod mailboxes;
pub mod messages;
