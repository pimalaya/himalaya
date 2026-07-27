#[cfg(any(feature = "imap", feature = "jmap", feature = "maildir"))]
pub mod attachment;
pub mod client;
pub mod crlf;
pub mod envelope;
pub mod flag;
pub mod mailbox;
pub mod message;
#[cfg(any(feature = "gmail", feature = "msgraph"))]
pub mod output;
#[cfg(any(feature = "imap", feature = "smtp"))]
pub mod raw;
