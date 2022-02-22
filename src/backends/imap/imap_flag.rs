use anyhow::{anyhow, Error, Result};
use std::{convert::TryFrom, fmt, ops::Deref};

/// Represents the imap flag variants.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum ImapFlag {
    Seen,
    Answered,
    Flagged,
    Deleted,
    Draft,
    Recent,
    MayCreate,
    Custom(String),
}

impl From<&str> for ImapFlag {
    fn from(flag_str: &str) -> Self {
        match flag_str {
            "seen" => ImapFlag::Seen,
            "answered" => ImapFlag::Answered,
            "flagged" => ImapFlag::Flagged,
            "deleted" => ImapFlag::Deleted,
            "draft" => ImapFlag::Draft,
            "recent" => ImapFlag::Recent,
            "maycreate" | "may-create" => ImapFlag::MayCreate,
            flag_str => ImapFlag::Custom(flag_str.into()),
        }
    }
}

impl TryFrom<&imap::types::Flag<'_>> for ImapFlag {
    type Error = Error;

    fn try_from(flag: &imap::types::Flag<'_>) -> Result<Self, Self::Error> {
        Ok(match flag {
            imap::types::Flag::Seen => ImapFlag::Seen,
            imap::types::Flag::Answered => ImapFlag::Answered,
            imap::types::Flag::Flagged => ImapFlag::Flagged,
            imap::types::Flag::Deleted => ImapFlag::Deleted,
            imap::types::Flag::Draft => ImapFlag::Draft,
            imap::types::Flag::Recent => ImapFlag::Recent,
            imap::types::Flag::MayCreate => ImapFlag::MayCreate,
            imap::types::Flag::Custom(custom) => ImapFlag::Custom(custom.to_string()),
            _ => return Err(anyhow!("cannot parse imap flag")),
        })
    }
}

/// Represents the imap flags.
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ImapFlags(pub Vec<ImapFlag>);

impl ImapFlags {
    /// Builds a symbols string
    pub fn to_symbols_string(&self) -> String {
        let mut flags = String::new();
        flags.push_str(if self.contains(&ImapFlag::Seen) {
            " "
        } else {
            "✷"
        });
        flags.push_str(if self.contains(&ImapFlag::Answered) {
            "↵"
        } else {
            " "
        });
        flags.push_str(if self.contains(&ImapFlag::Flagged) {
            "⚑"
        } else {
            " "
        });
        flags
    }
}

impl Deref for ImapFlags {
    type Target = Vec<ImapFlag>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for ImapFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut glue = "";

        for flag in &self.0 {
            write!(f, "{}", glue)?;
            match flag {
                ImapFlag::Seen => write!(f, "\\Seen")?,
                ImapFlag::Answered => write!(f, "\\Answered")?,
                ImapFlag::Flagged => write!(f, "\\Flagged")?,
                ImapFlag::Deleted => write!(f, "\\Deleted")?,
                ImapFlag::Draft => write!(f, "\\Draft")?,
                ImapFlag::Recent => write!(f, "\\Recent")?,
                ImapFlag::MayCreate => write!(f, "\\MayCreate")?,
                ImapFlag::Custom(custom) => write!(f, "{}", custom)?,
            }
            glue = " ";
        }

        Ok(())
    }
}

impl<'a> Into<Vec<imap::types::Flag<'a>>> for ImapFlags {
    fn into(self) -> Vec<imap::types::Flag<'a>> {
        self.0
            .into_iter()
            .map(|flag| match flag {
                ImapFlag::Seen => imap::types::Flag::Seen,
                ImapFlag::Answered => imap::types::Flag::Answered,
                ImapFlag::Flagged => imap::types::Flag::Flagged,
                ImapFlag::Deleted => imap::types::Flag::Deleted,
                ImapFlag::Draft => imap::types::Flag::Draft,
                ImapFlag::Recent => imap::types::Flag::Recent,
                ImapFlag::MayCreate => imap::types::Flag::MayCreate,
                ImapFlag::Custom(custom) => imap::types::Flag::Custom(custom.into()),
            })
            .collect()
    }
}

impl From<&str> for ImapFlags {
    fn from(flags_str: &str) -> Self {
        ImapFlags(
            flags_str
                .split_whitespace()
                .map(|flag_str| flag_str.trim().into())
                .collect(),
        )
    }
}

impl TryFrom<&[imap::types::Flag<'_>]> for ImapFlags {
    type Error = Error;

    fn try_from(flags: &[imap::types::Flag<'_>]) -> Result<Self, Self::Error> {
        let mut f = vec![];
        for flag in flags {
            f.push(flag.try_into()?);
        }
        Ok(Self(f))
    }
}
