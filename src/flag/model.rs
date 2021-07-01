pub(crate) use imap::types::Flag;
use serde::ser::{Serialize, SerializeSeq, Serializer};

use std::borrow::Cow;
use std::ops::{Deref, DerefMut};

/// Serializable wrapper for `imap::types::Flag`
#[derive(Debug, PartialEq)]
struct SerializableFlag<'flag>(&'flag imap::types::Flag<'flag>);

impl<'flag> Serialize for SerializableFlag<'flag> {
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
                _ => "Unknown",
            })
        }
}

/// This struct type includes all flags which belong to a given mail.
/// TODO: Use HashSet instead of vector
#[derive(Debug, PartialEq)]
pub struct Flags(Vec<Flag<'static>>);

impl Flags {
    pub fn new<'new>(flags: &[imap::types::Flag<'new>]) -> Self {
        Self(
            flags
            .iter()
            .map(|flag| convert_to_static(flag).unwrap())
            .collect::<Vec<Flag<'static>>>(),
            )
    }
}

impl ToString for Flags {
    fn to_string(&self) -> String {
        let mut flags = String::new();

        flags.push_str(if self.0.contains(&Flag::Seen) {
            " "
        } else {
            "✷"
        });

        flags.push_str(if self.0.contains(&Flag::Answered) {
            "↵"
        });

        flags.push_str(if self.0.contains(&Flag::Flagged) {
            "!"
        } else {
            " "
        });

        flags
    }
}

impl Deref for Flags {
    type Target = Vec<Flag<'static>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Flags {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Serialize for Flags {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mut seq = serializer.serialize_seq(Some(self.0.len()))?;

            for flag in &self.0 {
                seq.serialize_element(&SerializableFlag(flag))?;
            }

            seq.end()
        }
}

// =====================
// Helper Functions
// =====================
/// HINT: This function is only needed as long this pull request hasn't been
/// merged yet: https://github.com/jonhoo/rust-imap/pull/206
fn convert_to_static<'func>(flag: &'func Flag) -> Result<Flag<'static>, ()> {
    match flag {
        Flag::Seen => Ok(Flag::Seen),
        Flag::Answered => Ok(Flag::Answered),
        Flag::Flagged => Ok(Flag::Flagged),
        Flag::Deleted => Ok(Flag::Deleted),
        Flag::Draft => Ok(Flag::Draft),
        Flag::Recent => Ok(Flag::Recent),
        Flag::MayCreate => Ok(Flag::MayCreate),
        Flag::Custom(cow) => Ok(Flag::Custom(Cow::Owned(cow.to_string()))),
        &_ => Err(())
    }
}
