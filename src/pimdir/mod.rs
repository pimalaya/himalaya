//! # pimdir
//!
//! The `pimdir` command family and the adapter serving the unified
//! commands over a local pimdir store: an offline cache a sync engine
//! populates, not a live server.
//!
//! Reads go through [`PimdirReader`], the role that takes no lock and
//! carries no write, and are availability-aware: an item whose body is
//! not local still lists, and reads as not fetched rather than as an
//! error, which is the cue to sync.
//!
//! Writes are staged on the store's queue for its owner to apply and
//! push, the reader overlaying them on the next read. A staged creation
//! has no public id until applied, so no listing row: [`queue`] shows it.
//!
//! [`PimdirReader`]: io_pimdir::client::reader::PimdirReader

pub mod attachment;
pub mod backend;
pub mod cli;
pub mod client;
pub mod count;
pub mod flag;
pub mod index;
pub mod mailbox;
pub mod message;
pub mod performer;
pub mod queue;
pub mod search;
pub mod status;
pub mod tag;
pub mod thread;
