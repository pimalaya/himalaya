//! # Search
//!
//! The shared search query: a filter, a sort, the grammar parsing both
//! from one string, and the client-side evaluation run locally (filter
//! and sort for the local backends, the sort alone after a Graph search).

pub mod error;
#[cfg(any(feature = "maildir", feature = "mbox", feature = "pimdir"))]
pub mod eval;
pub mod filter;
pub mod parser;
pub mod query;
pub mod sort;
