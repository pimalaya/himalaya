//! pimdir backend: Himalaya over a local [pimdir](https://github.com/pimalaya/pimdir)
//! store — an offline **cache** (SQLite index + content-addressed blobs) that the
//! sync engine (Neverest) populates, not a live server.
//!
//! Reads project the store's shared items through [`PimdirReader`], the role
//! that takes no lock and carries no write, and are availability-aware: an item
//! whose body is not local (`level < Full`) lists fine but reads as "body not
//! fetched" rather than an error, the client's cue to sync. The reader overlays
//! the queue (pimdir SPEC §15.4), so a staged flag, move, copy or deletion shows
//! on the next read.
//!
//! Writes are **staged**: an action appended to the store's queue for its owner
//! to apply and push. A staged creation is the one write with nothing to show
//! for it in a listing, having no public id until the owner applies it, which is
//! what [`queue`] exists for.
//!
//! [`PimdirReader`]: io_pimdir::PimdirReader

pub mod backend;
pub mod cli;
pub mod client;
pub mod queue;
