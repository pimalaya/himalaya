use serde::Serialize;
use std::{fmt, ops};

use super::Flag;

/// Represents the list of flags.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct Flags(pub Vec<Flag>);

impl Flags {
    /// Builds a symbols string.
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

impl ops::Deref for Flags {
    type Target = Vec<Flag>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ops::DerefMut for Flags {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl fmt::Display for Flags {
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
                Flag::Custom(flag) => write!(f, "{}", flag)?,
            }
            glue = " ";
        }

        Ok(())
    }
}

impl From<&str> for Flags {
    fn from(flags: &str) -> Self {
        Flags(
            flags
                .split_whitespace()
                .map(|flag| flag.trim().into())
                .collect(),
        )
    }
}

impl FromIterator<Flag> for Flags {
    fn from_iter<T: IntoIterator<Item = Flag>>(iter: T) -> Self {
        let mut flags = Flags::default();
        for flag in iter {
            flags.push(flag);
        }
        flags
    }
}
