//! # Search
//!
//! The shared search query: a filter, a sort, the grammar parsing both
//! from one string, and the client-side evaluation the local backends
//! run it with.

pub mod error;
#[cfg(any(feature = "maildir", feature = "m2dir"))]
pub mod eval;
pub mod filter;
pub mod parser;
pub mod query;
pub mod sort;
