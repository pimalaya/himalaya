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
pub mod cli;

/// Here are the two **main structs** of this module: `Msg` and `Msgs` which
/// represent a *Mail* or *multiple Mails* in this crate.
pub mod model;

/// This module is used in the `Msg` struct, which should represent an
/// attachment of a msg.
pub mod attachment;

/// This module is used in the `Msg` struct, which should represent the headers
/// fields like `To:` and `From:`.
pub mod headers;

/// This module is used in the `Msg` struct, which should represent the body of
/// a msg; The part where you're writing some text like `Dear Mr. LMAO`.
pub mod body;
