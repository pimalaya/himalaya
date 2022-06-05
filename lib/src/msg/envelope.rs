use serde::Serialize;

use super::Flags;

/// Represents the message envelope. The envelope is just a message
/// subset, and is mostly used for listings.
#[derive(Debug, Default, Clone, Serialize)]
pub struct Envelope {
    /// Represents the message identifier.
    pub id: String,
    /// Represents the internal message identifier.
    pub internal_id: String,
    /// Represents the message flags.
    pub flags: Flags,
    /// Represents the subject of the message.
    pub subject: String,
    /// Represents the first sender of the message.
    pub sender: String,
    /// Represents the internal date of the message.
    pub date: Option<String>,
}
