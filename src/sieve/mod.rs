//! # ManageSieve
//!
//! The `sieve` command family, RFC 5804, one file per subcommand around
//! the protocol io-managesieve implements.

pub mod activate;
pub mod capability;
pub mod check;
pub mod cli;
pub mod client;
pub mod deactivate;
pub mod delete;
pub mod get;
pub mod list;
pub mod put;
pub mod raw;
pub mod rename;
pub mod script;
