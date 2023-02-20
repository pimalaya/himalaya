use serde::Serialize;

/// Represents the flag variants.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
pub enum Flag {
    Seen,
    Answered,
    Flagged,
    Deleted,
    Draft,
    Custom(String),
}

impl From<&himalaya_lib::Flag> for Flag {
    fn from(flag: &himalaya_lib::Flag) -> Self {
        match flag {
            himalaya_lib::Flag::Seen => Flag::Seen,
            himalaya_lib::Flag::Answered => Flag::Answered,
            himalaya_lib::Flag::Flagged => Flag::Flagged,
            himalaya_lib::Flag::Deleted => Flag::Deleted,
            himalaya_lib::Flag::Draft => Flag::Draft,
            himalaya_lib::Flag::Custom(flag) => Flag::Custom(flag.clone()),
        }
    }
}
