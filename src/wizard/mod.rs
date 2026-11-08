//! # Wizard
//!
//! The interactive configuration: one prompt discovering an account, a
//! review of what each domain picked, the sign-ins completing it, and the
//! document it comes out as.

pub mod configure;
pub mod discover;
#[cfg(feature = "gmail")]
pub mod gmail;
#[cfg(feature = "imap")]
pub mod imap;
#[cfg(feature = "jmap")]
pub mod jmap;
#[cfg(any(feature = "maildir", feature = "m2dir"))]
pub mod local;
#[cfg(feature = "msgraph")]
pub mod msgraph;
pub mod plan;
pub mod search;
pub mod secret;
pub mod server;
pub mod signin;
#[cfg(feature = "smtp")]
pub mod smtp;
