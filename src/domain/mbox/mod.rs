//! Module related to mailbox.

pub mod mbox_arg;
pub mod mbox_handler;

pub mod mbox_entity;
pub use mbox_entity::*;

mod mbox_type;
pub use mbox_type::{MboxType, MboxTypes};
