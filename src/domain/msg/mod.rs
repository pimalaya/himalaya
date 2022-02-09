//! This module holds everything which is related to a **Msg**/**Mail**. Here are
//! structs which **represent the data** in Msgs/Mails.

/// Includes the following subcommands:
/// - `list`
/// - `search`
/// - `write`
/// - `send`
/// - `save`
/// - `read`
/// - `attachments`
/// - `reply`
/// - `forward`
/// - `copy`
/// - `move`
/// - `delete`
/// - `template`
///
/// Execute `himalaya help <cmd>` where `<cmd>` is one entry of this list above
/// to get more information about them.
pub mod msg_arg;

pub mod msg_handler;
pub mod msg_utils;

pub mod flag_arg;
pub mod flag_handler;

pub mod flag_entity;
pub use flag_entity::*;

pub mod flags_entity;
pub use flags_entity::*;

pub mod envelope_entity;
pub use envelope_entity::*;

pub mod envelopes_entity;
pub use envelopes_entity::*;

pub mod tpl_arg;
pub use tpl_arg::TplOverride;

pub mod tpl_handler;

pub mod msg_entity;
pub use msg_entity::*;

pub mod parts_entity;
pub use parts_entity::*;

pub mod addr_entity;
pub use addr_entity::*;
