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

impl From<&email::email::Flag> for Flag {
    fn from(flag: &email::email::Flag) -> Self {
        use email::email::Flag::*;
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
