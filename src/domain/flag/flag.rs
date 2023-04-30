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

impl From<&pimalaya_email::Flag> for Flag {
    fn from(flag: &pimalaya_email::Flag) -> Self {
        match flag {
            pimalaya_email::Flag::Seen => Flag::Seen,
            pimalaya_email::Flag::Answered => Flag::Answered,
            pimalaya_email::Flag::Flagged => Flag::Flagged,
            pimalaya_email::Flag::Deleted => Flag::Deleted,
            pimalaya_email::Flag::Draft => Flag::Draft,
            pimalaya_email::Flag::Custom(flag) => Flag::Custom(flag.clone()),
        }
    }
}
