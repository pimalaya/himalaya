pub mod account;
pub mod backend;
pub mod cache;
pub mod cli;
pub mod completion;
pub mod config;
pub mod email;
pub mod folder;
pub mod manual;
pub mod output;
pub mod printer;
pub mod tracing;
pub mod ui;

#[doc(inline)]
pub use crate::email::{envelope, flag, message};
