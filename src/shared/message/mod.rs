// `arg` (the `MessageArg` input resolver) is used by the protocol
// send/save commands too, so it stays compiled. `compose`/`send` (and
// their `builder`/`handler` helpers, plus the `cli` enum that hosts
// them) also work over SMTP, so they carry `any(backend, feature =
// "smtp")`; the remaining commands need a mailbox backend, so they carry
// `backend`.
#[cfg(backend)]
pub mod add;
pub mod arg;
#[cfg(any(backend, feature = "smtp"))]
pub mod builder;
#[cfg(any(backend, feature = "smtp"))]
pub mod cli;
#[cfg(any(backend, feature = "smtp"))]
pub mod compose;
#[cfg(backend)]
pub mod copy;
#[cfg(backend)]
pub mod delete;
#[cfg(backend)]
pub mod forward;
#[cfg(any(backend, feature = "smtp"))]
pub mod handler;
#[cfg(backend)]
pub mod mv;
#[cfg(backend)]
pub mod read;
#[cfg(backend)]
pub mod reply;
#[cfg(any(backend, feature = "smtp"))]
pub mod send;
