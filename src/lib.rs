pub mod account;
pub mod cli;
pub mod completion;
pub mod config;
pub mod email;
pub mod folder;
pub mod manual;

#[doc(inline)]
pub use crate::email::{envelope, flag, message};
