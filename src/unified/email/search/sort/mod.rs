//! # Search sort
//!
//! The sort half of a search query: its chain, and its client-side
//! evaluation.

#[cfg(any(
    feature = "gmail",
    feature = "maildir",
    feature = "mbox",
    feature = "pimdir",
    feature = "msgraph"
))]
pub mod eval;
pub mod query;
