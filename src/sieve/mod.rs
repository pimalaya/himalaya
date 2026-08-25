//! ManageSieve subcommands (RFC 5804).
//!
//! The protocol lives in io-managesieve; this module is the CLI around
//! it, one file per subcommand.

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
