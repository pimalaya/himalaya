// `arg` (the `MessageArg` input resolver) is used by the protocol
// send/save commands too, so it stays compiled. `compose`/`send` (and
// their `builder`/`handler` helpers, plus the `cli` enum that hosts
// them) only need a send backend, so they carry the "any backend" gate;
// the remaining commands read/write storage and carry "any storage".
#[cfg(any(
    feature = "imap",
    feature = "jmap",
    feature = "gmail",
    feature = "msgraph",
    feature = "maildir",
    feature = "m2dir"
))]
pub mod add;
pub mod arg;
#[cfg(any(
    feature = "imap",
    feature = "jmap",
    feature = "gmail",
    feature = "msgraph",
    feature = "maildir",
    feature = "m2dir",
    feature = "smtp"
))]
pub mod builder;
#[cfg(any(
    feature = "imap",
    feature = "jmap",
    feature = "gmail",
    feature = "msgraph",
    feature = "maildir",
    feature = "m2dir",
    feature = "smtp"
))]
pub mod cli;
#[cfg(any(
    feature = "imap",
    feature = "jmap",
    feature = "gmail",
    feature = "msgraph",
    feature = "maildir",
    feature = "m2dir",
    feature = "smtp"
))]
pub mod compose;
#[cfg(any(
    feature = "imap",
    feature = "jmap",
    feature = "gmail",
    feature = "msgraph",
    feature = "maildir",
    feature = "m2dir"
))]
pub mod copy;
#[cfg(any(
    feature = "imap",
    feature = "jmap",
    feature = "gmail",
    feature = "msgraph",
    feature = "maildir",
    feature = "m2dir"
))]
pub mod forward;
#[cfg(any(
    feature = "imap",
    feature = "jmap",
    feature = "gmail",
    feature = "msgraph",
    feature = "maildir",
    feature = "m2dir",
    feature = "smtp"
))]
pub mod handler;
#[cfg(any(
    feature = "imap",
    feature = "jmap",
    feature = "gmail",
    feature = "msgraph",
    feature = "maildir",
    feature = "m2dir"
))]
pub mod mv;
#[cfg(any(
    feature = "imap",
    feature = "jmap",
    feature = "gmail",
    feature = "msgraph",
    feature = "maildir",
    feature = "m2dir"
))]
pub mod read;
#[cfg(any(
    feature = "imap",
    feature = "jmap",
    feature = "gmail",
    feature = "msgraph",
    feature = "maildir",
    feature = "m2dir"
))]
pub mod reply;
#[cfg(any(
    feature = "imap",
    feature = "jmap",
    feature = "gmail",
    feature = "msgraph",
    feature = "maildir",
    feature = "m2dir",
    feature = "smtp"
))]
pub mod send;
