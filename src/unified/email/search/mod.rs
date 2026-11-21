//! # Search
//!
//! The unified search query: a pimdir SEARCH §8 query read by io-pimdir,
//! lowered to the filter the server translations read, and the sort the
//! local backends and Graph run.

#[cfg(any(feature = "imap", feature = "gmail", feature = "msgraph"))]
pub mod error;
pub mod filter;
pub mod query;
pub mod sort;
