pub mod arg;
pub mod command;
pub mod config;

use serde::Serialize;
use std::{collections::HashSet, ops};

/// Represents the flag variants.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Ord, PartialOrd, Serialize)]
pub enum Flag {
    Seen,
    Answered,
    Flagged,
    Deleted,
    Draft,
    Custom(String),
}

impl From<&email::flag::Flag> for Flag {
    fn from(flag: &email::flag::Flag) -> Self {
        use email::flag::Flag::*;
        match flag {
            Seen => Flag::Seen,
            Answered => Flag::Answered,
            Flagged => Flag::Flagged,
            Deleted => Flag::Deleted,
            Draft => Flag::Draft,
            Custom(flag) => Flag::Custom(flag.clone()),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct Flags(pub HashSet<Flag>);

impl ops::Deref for Flags {
    type Target = HashSet<Flag>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<email::flag::Flags> for Flags {
    fn from(flags: email::flag::Flags) -> Self {
        Flags(flags.iter().map(Flag::from).collect())
    }
}
