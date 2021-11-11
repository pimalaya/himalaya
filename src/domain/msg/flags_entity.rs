use anyhow::{anyhow, Error, Result};
use serde::ser::{Serialize, SerializeSeq, Serializer};
use std::{
    borrow::Cow,
    collections::HashSet,
    convert::{TryFrom, TryInto},
    fmt::{self, Display},
    ops::{Deref, DerefMut},
};

use crate::domain::msg::{Flag, SerializableFlag};

/// Represents the flags of the message.
/// A hashset is used to avoid duplicates.
#[derive(Debug, Clone, Default)]
pub struct Flags(pub HashSet<Flag<'static>>);

impl Flags {
    /// Builds a symbols string based on flags contained in the hashset.
    pub fn to_symbols_string(&self) -> String {
        let mut flags = String::new();
        flags.push_str(if self.contains(&Flag::Seen) {
            " "
        } else {
            "✷"
        });
        flags.push_str(if self.contains(&Flag::Answered) {
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

impl Display for Flags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut glue = "";

        for flag in &self.0 {
            write!(f, "{}", glue)?;
            match flag {
                Flag::Seen => write!(f, "\\Seen")?,
                Flag::Answered => write!(f, "\\Answered")?,
                Flag::Flagged => write!(f, "\\Flagged")?,
                Flag::Deleted => write!(f, "\\Deleted")?,
                Flag::Draft => write!(f, "\\Draft")?,
                Flag::Recent => write!(f, "\\Recent")?,
                Flag::MayCreate => write!(f, "\\MayCreate")?,
                Flag::Custom(cow) => write!(f, "{}", cow)?,
                _ => (),
            }
            glue = " ";
        }

        Ok(())
    }
}

impl<'a> TryFrom<Vec<Flag<'a>>> for Flags {
    type Error = Error;

    fn try_from(flags: Vec<Flag<'a>>) -> Result<Flags> {
        let mut set: HashSet<Flag<'static>> = HashSet::new();

        for flag in flags {
            set.insert(match flag {
                Flag::Seen => Flag::Seen,
                Flag::Answered => Flag::Answered,
                Flag::Flagged => Flag::Flagged,
                Flag::Deleted => Flag::Deleted,
                Flag::Draft => Flag::Draft,
                Flag::Recent => Flag::Recent,
                Flag::MayCreate => Flag::MayCreate,
                Flag::Custom(cow) => Flag::Custom(Cow::Owned(cow.to_string())),
                flag => return Err(anyhow!(r#"cannot parse flag "{}""#, flag)),
            });
        }

        Ok(Self(set))
    }
}

impl<'a> TryFrom<&'a [Flag<'a>]> for Flags {
    type Error = Error;

    fn try_from(flags: &'a [Flag<'a>]) -> Result<Flags> {
        flags.to_vec().try_into()
    }
}

impl Deref for Flags {
    type Target = HashSet<Flag<'static>>;

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

impl<'a> From<Vec<&'a str>> for Flags {
    fn from(flags: Vec<&'a str>) -> Self {
        let mut map: HashSet<Flag<'static>> = HashSet::new();

        for f in flags {
            match f.to_lowercase().as_str() {
                "answered" => map.insert(Flag::Answered),
                "deleted" => map.insert(Flag::Deleted),
                "draft" => map.insert(Flag::Draft),
                "flagged" => map.insert(Flag::Flagged),
                "maycreate" => map.insert(Flag::MayCreate),
                "recent" => map.insert(Flag::Recent),
                "seen" => map.insert(Flag::Seen),
                custom => map.insert(Flag::Custom(Cow::Owned(custom.into()))),
            };
        }

        Self(map)
    }
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
