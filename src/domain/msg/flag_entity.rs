pub use imap::types::Flag;
use serde::ser::{Serialize, Serializer};

/// Serializable wrapper arround [`imap::types::Flag`].
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SerializableFlag<'a>(pub &'a Flag<'a>);

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
