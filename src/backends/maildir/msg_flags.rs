use anyhow::{anyhow, Error, Result};
use std::convert::{TryFrom, TryInto};

pub struct Flags(Vec<Flag>);

impl ToString for Flags {
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

impl TryFrom<&str> for Flags {
    type Error = Error;

    fn try_from(flags_str: &str) -> Result<Self, Self::Error> {
        let mut flags = vec![];
        for flag_str in flags_str.split_whitespace() {
            flags.push(flag_str.trim().try_into()?);
        }
        Ok(Flags(flags))
    }
}

pub enum Flag {
    Replied,
    Deleted,
    Draft,
    Flagged,
    Seen,
}

impl Into<char> for &Flag {
    fn into(self) -> char {
        match self {
            Flag::Replied => 'R',
            Flag::Deleted => 'T',
            Flag::Draft => 'D',
            Flag::Flagged => 'F',
            Flag::Seen => 'S',
        }
    }
}

impl TryFrom<&str> for Flag {
    type Error = Error;

    fn try_from(flag_str: &str) -> Result<Self, Self::Error> {
        match flag_str {
            "replied" => Ok(Flag::Replied),
            "deleted" => Ok(Flag::Deleted),
            "draft" => Ok(Flag::Draft),
            "flagged" => Ok(Flag::Flagged),
            "seen" => Ok(Flag::Seen),
            _ => Err(anyhow!("cannot parse flag {:?}", flag_str)),
        }
    }
}
