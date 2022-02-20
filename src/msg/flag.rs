use std::{
    collections::HashSet,
    ops::{Deref, DerefMut},
};

/// Represents the flags of the message.
/// A hashset is used to avoid duplicates.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct Flags(pub HashSet<Flag>);

impl Flags {
    /// Builds a symbols string based on flags contained in the hashset.
    pub fn to_symbols_string(&self) -> String {
        let mut flags = String::new();
        flags.push_str(if self.contains(&Flag::Seen) {
            " "
        } else {
            "✷"
        });
        flags.push_str(if self.contains(&Flag::Replied) {
            "↵"
        } else {
            " "
        });
        flags.push_str(if self.contains(&Flag::Flagged) {
            "⚑"
        } else {
            " "
        });
        flags
    }
}

impl<'a> From<Vec<RawImapFlag<'a>>> for Flags {
    fn from(raw_flags: Vec<RawImapFlag<'a>>) -> Flags {
        Self(
            raw_flags
                .iter()
                .filter_map(|raw_flag| match raw_flag {
                    RawImapFlag::Seen => Some(Flag::Seen),
                    RawImapFlag::Answered => Some(Flag::Replied),
                    RawImapFlag::Flagged => Some(Flag::Flagged),
                    RawImapFlag::Deleted => Some(Flag::Trashed),
                    RawImapFlag::Draft => Some(Flag::Draft),
                    RawImapFlag::Custom(custom) => Some(Flag::Custom(custom.to_string())),
                    _ => None,
                })
                .collect(),
        )
    }
}

impl<'a> From<&'a [RawImapFlag<'a>]> for Flags {
    fn from(flags: &'a [RawImapFlag<'a>]) -> Flags {
        flags.to_vec().into()
    }
}

impl Deref for Flags {
    type Target = HashSet<Flag>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Flags {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a> From<Vec<&'a str>> for Flags {
    fn from(flags_str: Vec<&'a str>) -> Self {
        Self(
            flags_str
                .iter()
                .map(|flag_str| match flag_str.to_lowercase().as_str() {
                    "seen" => Flag::Seen,
                    "answered" | "replied" => Flag::Replied,
                    "deleted" | "trashed" => Flag::Trashed,
                    "draft" => Flag::Draft,
                    "flagged" => Flag::Flagged,
                    custom => Flag::Custom(custom.into()),
                })
                .collect(),
        )
    }
}

impl From<&str> for Flags {
    fn from(flags: &str) -> Self {
        flags.split(" ").collect::<Vec<_>>().into()
    }
}

type RawImapFlag<'a> = imap::types::Flag<'a>;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
pub enum Flag {
    Seen,
    Replied,
    Passed,
    Flagged,
    Trashed,
    Draft,
    Custom(String),
}

// FIXME
//#[cfg(test)]
//mod tests {
//    use crate::domain::msg::flag::entity::Flags;
//    use imap::types::Flag;
//    use std::collections::HashSet;

//    #[test]
//    fn test_get_signs() {
//        let flags = Flags::from(vec![Flag::Seen, Flag::Answered]);

//        assert_eq!(flags.to_symbols_string(), " ↵ ".to_string());
//    }

//    #[test]
//    fn test_from_string() {
//        let flags = Flags::from("Seen Answered");

//        let expected = Flags::from(vec![Flag::Seen, Flag::Answered]);

//        assert_eq!(flags, expected);
//    }

//    #[test]
//    fn test_to_string() {
//        let flags = Flags::from(vec![Flag::Seen, Flag::Answered]);

//        // since we can't influence the order in the HashSet, we're gonna convert it into a vec,
//        // sort it according to the names and compare it aftwards.
//        let flag_string = flags.to_string();
//        let mut flag_vec: Vec<String> = flag_string
//            .split_ascii_whitespace()
//            .map(|word| word.to_string())
//            .collect();
//        flag_vec.sort();

//        assert_eq!(
//            flag_vec,
//            vec!["\\Answered".to_string(), "\\Seen".to_string()]
//        );
//    }

//    #[test]
//    fn test_from_vec() {
//        let flags = Flags::from(vec![Flag::Seen, Flag::Answered]);

//        let mut expected = HashSet::new();
//        expected.insert(Flag::Seen);
//        expected.insert(Flag::Answered);

//        assert_eq!(flags.0, expected);
//    }
//}
