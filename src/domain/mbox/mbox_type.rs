use std::{
    convert::From,
    collections::HashSet,
    fmt,
};

use imap::types::NameAttribute;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MboxType {
    Sent,
    Trash,
    Important,
    Junk,
    Drafts,
    Flagged,
    Other,
}

impl From<&NameAttribute<'_>> for MboxType {
    fn from(name_attribute: &NameAttribute<'_>) -> Self {

        if let NameAttribute::Custom(cow) = name_attribute {
            match cow.to_string().as_ref() {
                "\\Sent" => return MboxType::Sent,
                "\\Trash" => return MboxType::Trash,
                "\\Important" => return MboxType::Important,
                "\\Junk" => return MboxType::Junk,
                "\\Drafts" => return MboxType::Drafts,
                "\\Flagged" => return MboxType::Flagged,
                _ => (),
            }
        }
        Self::Other
    }
}

impl fmt::Display for MboxType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sent => write!(formatter, "Sent"),
            Self::Trash => write!(formatter, "Trash"),
            Self::Important => write!(formatter, "Important"),
            Self::Junk => write!(formatter, "Junk"),
            Self::Drafts => write!(formatter, "Drafts"),
            Self::Flagged => write!(formatter, "Flagged"),
            Self::Other => write!(formatter, "Other"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MboxTypes(pub Vec<MboxType>);

impl MboxTypes {
    pub fn contains(&self, mbox_type: MboxType) -> bool {
        self.0.contains(&mbox_type)
    }
}

impl From<HashSet<NameAttribute<'_>>> for MboxTypes {
    fn from(name_attributes: HashSet<NameAttribute<'_>>) -> Self {
        let attribute_types = name_attributes
             .iter()
             .map(|name_attribute| MboxType::from(name_attribute))
             .collect();

        Self(attribute_types)
    }
}
