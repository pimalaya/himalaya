//! # pimdir
//!
//! The `pimdir` command family and the adapter serving the shared
//! commands over a local pimdir store: an offline cache a sync engine
//! populates, not a live server.
//!
//! Reads go through [`PimdirReader`], the role that takes no lock and
//! carries no write, and are availability-aware: an item whose body is
//! not local still lists, and reads as not fetched rather than as an
//! error, which is the cue to sync.
//!
//! Writes are staged as actions appended to the store's queue for its
//! owner to apply and push, the reader overlaying them so they show on
//! the next read. A staged creation has no public id until the owner
//! applies it, and so no row in a listing, which is what [`queue`] is
//! for.
//!
//! [`PimdirReader`]: io_pimdir::PimdirReader

pub mod backend;
pub mod cli;
pub mod client;
pub mod queue;
