//! # Search
//!
//! The shared search query: a filter, a sort, the grammar parsing both
//! from one string, and the client-side evaluation run locally
//! (filter + sort for local backends, sort-after-search for Graph).

pub mod error;
#[cfg(any(
    feature = "maildir",
    feature = "m2dir",
    feature = "mbox",
    feature = "pimdir",
    feature = "msgraph"
))]
pub mod eval;
pub mod filter;
pub mod parser;
pub mod query;
pub mod sort;
