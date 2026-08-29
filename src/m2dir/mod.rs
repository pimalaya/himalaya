//! # m2dir
//!
//! The `m2dir` command family, covering what maps onto the on-disk
//! layout, plus the adapter serving the shared commands over m2dir.

pub mod arg;
pub mod backend;
pub mod cli;
pub mod client;
pub mod create;
pub mod delete;
pub mod flag;
pub mod list;
pub mod message;
