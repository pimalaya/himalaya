pub mod account;
pub mod backend;
pub mod cache;
pub mod cli;
pub mod completion;
pub mod config;
pub mod email;
pub mod folder;
#[cfg(feature = "imap")]
pub mod imap;
#[cfg(feature = "maildir")]
pub mod maildir;
pub mod manual;
#[cfg(feature = "notmuch")]
pub mod notmuch;
pub mod output;
pub mod printer;
#[cfg(feature = "sendmail")]
pub mod sendmail;
#[cfg(feature = "smtp")]
pub mod smtp;
pub mod ui;

#[doc(inline)]
// pub use email::{envelope, flag, message, template};
pub use email::{envelope, flag, message};
