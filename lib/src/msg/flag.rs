use serde::Serialize;

/// Represents the flag variants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Flag {
    Seen,
    Answered,
    Flagged,
    Deleted,
    Draft,
    Recent,
    Custom(String),
}

impl From<&str> for Flag {
    fn from(flag_str: &str) -> Self {
        match flag_str {
            "seen" => Flag::Seen,
            "answered" | "replied" => Flag::Answered,
            "flagged" => Flag::Flagged,
            "deleted" | "trashed" => Flag::Deleted,
            "draft" => Flag::Draft,
            "recent" => Flag::Recent,
            flag => Flag::Custom(flag.into()),
        }
    }
}
