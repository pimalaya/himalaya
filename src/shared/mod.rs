#[cfg(any(
    feature = "imap",
    feature = "jmap",
    feature = "gmail",
    feature = "msgraph",
    feature = "maildir",
    feature = "m2dir",
))]
pub mod attachment;
#[cfg(any(
    feature = "imap",
    feature = "jmap",
    feature = "gmail",
    feature = "msgraph",
    feature = "maildir",
    feature = "m2dir",
    feature = "smtp",
))]
pub mod client;
pub mod crlf;
#[cfg(any(
    feature = "imap",
    feature = "jmap",
    feature = "gmail",
    feature = "msgraph",
    feature = "maildir",
    feature = "m2dir",
))]
pub mod envelope;
#[cfg(any(
    feature = "imap",
    feature = "jmap",
    feature = "gmail",
    feature = "msgraph",
    feature = "maildir",
    feature = "m2dir",
))]
pub mod flag;
#[cfg(any(
    feature = "imap",
    feature = "jmap",
    feature = "gmail",
    feature = "msgraph",
    feature = "maildir",
    feature = "m2dir",
))]
pub mod mailbox;
pub mod message;
#[cfg(any(feature = "gmail", feature = "msgraph"))]
pub mod output;
#[cfg(any(feature = "imap", feature = "smtp"))]
pub mod raw;
