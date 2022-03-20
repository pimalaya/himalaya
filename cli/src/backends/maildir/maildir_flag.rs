use anyhow::{anyhow, Error, Result};
use std::{
    convert::{TryFrom, TryInto},
    ops::Deref,
};

/// Represents the maildir flag variants.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum MaildirFlag {
    Passed,
    Replied,
    Seen,
    Trashed,
    Draft,
    Flagged,
    Custom(char),
}

/// Represents the maildir flags.
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize)]
pub struct MaildirFlags(pub Vec<MaildirFlag>);

impl MaildirFlags {
    /// Builds a symbols string
    pub fn to_symbols_string(&self) -> String {
        let mut flags = String::new();
        flags.push_str(if self.contains(&MaildirFlag::Seen) {
            " "
        } else {
            "✷"
        });
        flags.push_str(if self.contains(&MaildirFlag::Replied) {
            "↵"
        } else {
            " "
        });
        flags.push_str(if self.contains(&MaildirFlag::Passed) {
            "↗"
        } else {
            " "
        });
        flags.push_str(if self.contains(&MaildirFlag::Flagged) {
            "⚑"
        } else {
            " "
        });
        flags
    }
}

impl Deref for MaildirFlags {
    type Target = Vec<MaildirFlag>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ToString for MaildirFlags {
    fn to_string(&self) -> String {
        self.0
            .iter()
            .map(|flag| {
                let flag_char: char = flag.into();
                flag_char
            })
            .collect()
    }
}

impl TryFrom<&str> for MaildirFlags {
    type Error = Error;

    fn try_from(flags_str: &str) -> Result<Self, Self::Error> {
        let mut flags = vec![];
        for flag_str in flags_str.split_whitespace() {
            flags.push(flag_str.trim().try_into()?);
        }
        Ok(MaildirFlags(flags))
    }
}

impl From<&maildir::MailEntry> for MaildirFlags {
    fn from(mail_entry: &maildir::MailEntry) -> Self {
        let mut flags = vec![];
        for c in mail_entry.flags().chars() {
            flags.push(match c {
                'P' => MaildirFlag::Passed,
                'R' => MaildirFlag::Replied,
                'S' => MaildirFlag::Seen,
                'T' => MaildirFlag::Trashed,
                'D' => MaildirFlag::Draft,
                'F' => MaildirFlag::Flagged,
                custom => MaildirFlag::Custom(custom),
            })
        }
        Self(flags)
    }
}

impl Into<char> for &MaildirFlag {
    fn into(self) -> char {
        match self {
            MaildirFlag::Passed => 'P',
            MaildirFlag::Replied => 'R',
            MaildirFlag::Seen => 'S',
            MaildirFlag::Trashed => 'T',
            MaildirFlag::Draft => 'D',
            MaildirFlag::Flagged => 'F',
            MaildirFlag::Custom(custom) => *custom,
        }
    }
}

impl TryFrom<&str> for MaildirFlag {
    type Error = Error;

    fn try_from(flag_str: &str) -> Result<Self, Self::Error> {
        match flag_str {
            "passed" => Ok(MaildirFlag::Passed),
            "replied" => Ok(MaildirFlag::Replied),
            "seen" => Ok(MaildirFlag::Seen),
            "trashed" => Ok(MaildirFlag::Trashed),
            "draft" => Ok(MaildirFlag::Draft),
            "flagged" => Ok(MaildirFlag::Flagged),
            flag_str => Err(anyhow!("cannot parse maildir flag {:?}", flag_str)),
        }
    }
}
