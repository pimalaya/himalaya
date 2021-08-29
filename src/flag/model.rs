pub(crate) use imap::types::Flag;
use serde::ser::{Serialize, SerializeSeq, Serializer};

use std::borrow::Cow;
use std::collections::HashSet;
use std::ops::{Deref, DerefMut};

use std::convert::From;

/// Serializable wrapper for `imap::types::Flag`
#[derive(Debug, PartialEq, Eq, Clone)]
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
/// It's used in the [`Msg.flags`] attribute field of the `Msg` struct. To be more clear: It's just
/// a wrapper for the [`imap::types::Flag`] but without a lifetime.
///
/// [`Msg.flags`]: struct.Msg.html#structfield.flags
/// [`imap::types::Flag`]: https://docs.rs/imap/2.4.1/imap/types/enum.Flag.html
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct Flags(pub HashSet<Flag<'static>>);

impl Flags {

    /// Returns the flags of their respective flag value in the following order:
    ///
    /// 1. Seen
    /// 2. Answered
    /// 3. Flagged
    pub fn get_signs(&self) -> String {
        let mut flags = String::new();

        flags.push_str(if self.0.contains(&Flag::Seen) {
            " "
        } else {
            "✷"
        });

        flags.push_str(if self.0.contains(&Flag::Answered) {
            "↵"
        } else {
            " "
        });

        flags.push_str(if self.0.contains(&Flag::Flagged) {
            "!"
        } else {
            " "
        });

        flags
    }
}

impl ToString for Flags {
    fn to_string(&self) -> String {

        let mut flags = String::new();

        for flag in &self.0 {
            match flag {
                Flag::Seen => flags.push_str("\\Seen "),
                Flag::Answered => flags.push_str("\\Answered "),
                Flag::Flagged => flags.push_str("\\Flagged "),
                Flag::Deleted => flags.push_str("\\Deleted "),
                Flag::Draft => flags.push_str("\\Draft "),
                Flag::Recent => flags.push_str("\\Recent "),
                Flag::MayCreate => flags.push_str("\\MayCreate "),
                Flag::Custom(cow) => flags.push_str(&format!("\\{} ", cow)),
                _ => panic!("Unknown flag!"),
            }
        }

        // remove the trailing whitespace
        flags.pop();
        flags
    }
}

impl<'a> From<&[imap::types::Flag<'a>]> for Flags {
    fn from(flags: &[imap::types::Flag<'a>]) -> Self {
        Self(
            flags
                .iter()
                .map(|flag| convert_to_static(flag).unwrap())
                .collect::<HashSet<Flag<'static>>>(),
        )
    }
}

impl<'a> From<Vec<imap::types::Flag<'a>>> for Flags {
    fn from(flags: Vec<imap::types::Flag<'a>>) -> Self {
        Self (
            flags
                .iter()
                .map(|flag| convert_to_static(flag).unwrap())
                .collect::<HashSet<Flag<'static>>>()
        )
    }
}

/// Converst a string of flags into their appropriate flag representation. For example `"Seen"` is
/// gonna be convertred to `Flag::Seen`.
///
/// # Example
/// ```rust
/// use himalaya::flag::model::Flags;
/// use imap::types::Flag;
/// use std::collections::HashSet;
///
/// fn main() {
///     let flags = "Seen Answered";
///
///     let mut expected = HashSet::new();
///     expected.insert(Flag::Seen);
///     expected.insert(Flag::Answered);
///
///     let output = Flags::from(flags);
///
///     assert_eq!(output.0, expected);
/// }
/// ```
impl From<&str> for Flags {
    fn from(flags: &str) -> Self {
        let mut content: HashSet<Flag<'static>> = HashSet::new();

        for flag in flags.split_ascii_whitespace() {
            match flag {
                "Seen" => content.insert(Flag::Seen),
                "Answered" => content.insert(Flag::Answered),
                "Deleted" => content.insert(Flag::Flagged),
                "Draft" => content.insert(Flag::Draft),
                "Recent" => content.insert(Flag::Recent),
                "MayCreate" => content.insert(Flag::MayCreate),
                _other => content.insert(Flag::Custom(Cow::Owned(_other.to_string()))),
            };
        }

        Self(content)
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

// == Helper Functions ==
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
        &_ => Err(()),
    }
}

#[cfg(test)]
mod tests {

    use imap::types::Flag;
    use crate::flag::model::Flags;
    use std::collections::HashSet;

    #[test]
    fn test_get_signs() {
        let flags = Flags::from(vec![Flag::Seen, Flag::Answered]);

        assert_eq!(flags.get_signs(), " ↵ ".to_string());
    }

    #[test]
    fn test_from_string() {
        let flags = Flags::from("Seen Answered");

        let expected = Flags::from(vec![Flag::Seen, Flag::Answered]);

        assert_eq!(flags, expected);
    }

    #[test]
    fn test_to_string() {
        let flags = Flags::from(vec![Flag::Seen, Flag::Answered]);

        assert_eq!(flags.to_string(), "\\Answered \\Seen");
    }

    #[test]
    fn test_from_vec() {
        let flags = Flags::from(vec![Flag::Seen, Flag::Answered]);

        let mut expected = HashSet::new();
        expected.insert(Flag::Seen);
        expected.insert(Flag::Answered);

        assert_eq!(flags.0, expected);
    }
}
