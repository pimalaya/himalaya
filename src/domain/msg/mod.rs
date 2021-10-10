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
pub mod arg;

pub mod handler;
pub mod utils;

pub mod flag;
pub use flag::*;

pub mod envelope;
pub use envelope::*;

pub mod tpl;
pub use tpl::*;

pub mod entity_msg;
pub use entity_msg::*;

pub mod entity_parts;
pub use entity_parts::*;
