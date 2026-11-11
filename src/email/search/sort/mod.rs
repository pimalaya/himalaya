//! # Search sort
//!
//! The sort half of a search query: its AST, the grammar parsing one
//! from a string, and its client-side evaluation.

#[cfg(any(
    feature = "maildir",
    feature = "m2dir",
    feature = "mbox",
    feature = "pimdir",
    feature = "msgraph"
))]
pub mod eval;
pub mod parser;
pub mod query;
