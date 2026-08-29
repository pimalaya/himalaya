//! # Wizard
//!
//! The interactive configuration: one prompt discovering an account, the
//! per-backend prompts completing it, and the document it comes out as.

pub mod configure;
pub mod discover;
#[cfg(feature = "gmail")]
pub mod gmail;
#[cfg(all(feature = "imap", feature = "smtp"))]
pub mod imap_smtp;
#[cfg(feature = "jmap")]
pub mod jmap;
#[cfg(any(feature = "maildir", feature = "m2dir"))]
pub mod local;
#[cfg(any(
    feature = "imap",
    feature = "jmap",
    feature = "gmail",
    feature = "msgraph"
))]
pub mod mailbox;
#[cfg(feature = "msgraph")]
pub mod msgraph;
pub mod search;
#[cfg(any(
    feature = "imap",
    feature = "jmap",
    feature = "gmail",
    feature = "msgraph"
))]
pub mod secret;
