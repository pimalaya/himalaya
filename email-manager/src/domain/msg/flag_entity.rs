pub use imap::types::Flag;
use serde::ser::{Serialize, Serializer};

/// Represents a serializable `imap::types::Flag`.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SerializableFlag<'a>(pub &'a Flag<'a>);

/// Implements the serialize trait for `imap::types::Flag`.
/// Remote serialization cannot be used because of the [#[non_exhaustive]] directive of
/// `imap::types::Flag`.
///
/// [#[non_exhaustive]]: https://github.com/serde-rs/serde/issues/1991
impl<'a> Serialize for SerializableFlag<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(match self.0 {
            Flag::Seen => "Seen",
            Flag::Answered => "Answered",
            Flag::Flagged => "Flagged",
            Flag::Deleted => "Deleted",
            Flag::Draft => "Draft",
            Flag::Recent => "Recent",
            Flag::MayCreate => "MayCreate",
            Flag::Custom(cow) => cow,
            // TODO: find a way to return an error
            _ => "Unknown",
        })
    }
}
