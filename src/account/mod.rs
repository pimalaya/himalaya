//! # Account
//!
//! The `account` command family: inspecting the accounts a configuration
//! declares, and the merged runtime account every other command consumes.

pub mod capability;
pub mod check;
pub mod cli;
#[cfg(any(backend, feature = "smtp", feature = "sieve"))]
pub mod context;
pub mod list;
