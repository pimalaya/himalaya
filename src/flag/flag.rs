use serde::ser::{Serialize, Serializer};

use std::borrow::Cow;

// =========
// Enum
// =========
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Flag {
    Seen,
    Answered,
    Flagged,
    Deleted,
    Draft,
    Recent,
    MayCreate,
    Custom(String),
}

impl Flag {
    pub fn convert(flags: &[imap::types::Flag<'_>]) -> Vec<Self> {

        let mut new_flags = Vec::new();

        for flag in flags {
            new_flags.push(Flag::from(flag));
        }

        new_flags
    }

    pub fn revert(&self) -> imap::types::Flag {
        match self {
            Flag::Seen => imap::types::Flag::Seen,
            Flag::Answered => imap::types::Flag::Answered,
            Flag::Flagged => imap::types::Flag::Flagged,
            Flag::Deleted => imap::types::Flag::Deleted,
            Flag::Draft => imap::types::Flag::Draft,
            Flag::Recent => imap::types::Flag::Recent,
            Flag::MayCreate => imap::types::Flag::MayCreate,
            Flag::Custom(custom) =>  imap::types::Flag::Custom(Cow::from(custom)),
        }
    }
}

// ===========
// Traits
// ===========
impl From<&imap::types::Flag<'_>> for Flag {
    fn from(flag: &imap::types::Flag<'_>) -> Self {
        match flag {
            imap::types::Flag::Seen => Flag::Seen,
            imap::types::Flag::Answered => Flag::Answered,
            imap::types::Flag::Flagged => Flag::Flagged,
            imap::types::Flag::Deleted => Flag::Deleted,
            imap::types::Flag::Draft => Flag::Draft,
            imap::types::Flag::Recent => Flag::Recent,
            imap::types::Flag::MayCreate => Flag::MayCreate,
            imap::types::Flag::Custom(cow) => Flag::Custom(cow.clone().into_owned()),
        }
    }
}

impl Serialize for Flag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            match self {
                Flag::Seen        => serializer.serialize_unit_variant("Flag", 0, "Seen"),
                Flag::Answered    => serializer.serialize_unit_variant("Flag", 1, "Answered"),
                Flag::Flagged     => serializer.serialize_unit_variant("Flag", 2, "Flagged"),
                Flag::Deleted     => serializer.serialize_unit_variant("Flag", 3, "Deleted"),
                Flag::Draft       => serializer.serialize_unit_variant("Flag", 4, "Draft"),
                Flag::Recent      => serializer.serialize_unit_variant("Flag", 5, "Recent"),
                Flag::MayCreate   => serializer.serialize_unit_variant("Flag", 6, "MayCreate"),
                Flag::Custom(custom) => serializer.serialize_unit_variant("Flag", 7, &custom),
            }
        }
}
