//! # Vacation
//!
//! The `vacation` command family, reading and setting the account's
//! auto-reply on the backend keeping it.

#![cfg_attr(not(any(feature = "jmap", feature = "gmail")), allow(dead_code))]

pub mod cli;
pub mod get;
pub mod set;
