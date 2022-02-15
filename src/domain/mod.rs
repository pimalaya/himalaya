//! Domain-specific modules.

pub mod imap;
pub use self::imap::*;

pub mod maildir;
pub use self::maildir::*;

pub mod mbox;
pub use mbox::*;

pub mod msg;
pub use msg::*;

pub mod smtp;
pub use smtp::*;
