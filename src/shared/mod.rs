// `backend` (see build.rs) is set when any mailbox backend is enabled.
// The read/write shared commands need one; `client` (and the send-only
// `messages compose`/`send`) also work over SMTP, hence
// `any(backend, feature = "smtp")`. `crlf` is pure and always compiled;
// `message` stays compiled for its `arg` submodule.
#[cfg(backend)]
pub mod attachment;
#[cfg(any(backend, feature = "smtp"))]
pub mod client;
pub mod crlf;
#[cfg(backend)]
pub mod envelope;
#[cfg(backend)]
pub mod flag;
#[cfg(backend)]
pub mod mailbox;
pub mod message;
#[cfg(any(feature = "gmail", feature = "msgraph"))]
pub mod output;
#[cfg(any(feature = "imap", feature = "smtp"))]
pub mod raw;
