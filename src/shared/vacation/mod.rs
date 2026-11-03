//! # Vacation
//!
//! The `vacation` command family, reading and setting the account's
//! auto-reply on the backend keeping it.

// NOTE: a build with neither JMAP nor Gmail parses the commands and
// refuses them before they run, so nothing here is reached.
#![cfg_attr(not(any(feature = "jmap", feature = "gmail")), allow(dead_code))]

pub mod cli;
pub mod get;
pub mod set;
