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

impl From<&email::flag::Flag> for Flag {
    fn from(flag: &email::flag::Flag) -> Self {
        use email::flag::Flag::*;
        match flag {
            Seen => Flag::Seen,
            Answered => Flag::Answered,
            Flagged => Flag::Flagged,
            Deleted => Flag::Deleted,
            Draft => Flag::Draft,
            Custom(flag) => Flag::Custom(flag.clone()),
        }
    }
}
