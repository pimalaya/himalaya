pub(crate) use imap::types::Flag;
use serde::ser::{Serialize, SerializeSeq, Serializer};
use std::ops::Deref;

// Serializable wrapper for `imap::types::Flag`

#[derive(Debug, PartialEq)]
struct SerializableFlag<'f>(&'f imap::types::Flag<'f>);

impl<'f> Serialize for SerializableFlag<'f> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(match &self.0 {
            Flag::Seen => "Seen",
            Flag::Answered => "Answered",
            Flag::Flagged => "Flagged",
            Flag::Deleted => "Deleted",
            Flag::Draft => "Draft",
            Flag::Recent => "Recent",
            Flag::MayCreate => "MayCreate",
            Flag::Custom(cow) => cow,
        })
    }
}

// Flags

#[derive(Debug, PartialEq)]
pub struct Flags<'f>(&'f [Flag<'f>]);

impl<'f> Flags<'f> {
    pub fn new(flags: &'f [imap::types::Flag<'f>]) -> Self {
        Self(flags)
    }
}

impl<'f> ToString for Flags<'f> {
    fn to_string(&self) -> String {
        let mut flags = String::new();

        flags.push_str(if self.0.contains(&Flag::Seen) {
            " "
        } else {
            "🟓"
        });

        flags.push_str(if self.0.contains(&Flag::Answered) {
            "↩"
        } else {
            " "
        });

        flags.push_str(if self.0.contains(&Flag::Flagged) {
            "!"
        } else {
            " "
        });

        flags
    }
}

impl<'f> Deref for Flags<'f> {
    type Target = &'f [Flag<'f>];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'f> Serialize for Flags<'f> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;

        for flag in self.0 {
            seq.serialize_element(&SerializableFlag(flag))?;
        }

        seq.end()
    }
}
