//! pimdir backend: Himalaya over a local [pimdir](https://github.com/pimalaya/pimdir)
//! store — an offline **cache** (SQLite index + content-addressed blobs) that the
//! sync engine (Neverest) populates, not a live server.
//!
//! Reads project the store's shared items ([`io_pimdir`]'s client read API) and
//! are availability-aware: an item whose body is not local (`level < Full`) lists
//! fine but reads as "body not fetched" rather than an error — the client's cue to
//! sync. Writes are **staged** io-replica mutations a later sync propagates; they
//! are attributed to the configured [`source`](crate::config::PimdirConfig::source),
//! which must match the sync source for the change to reach the servers.

pub mod backend;
pub mod client;
pub mod hash;
