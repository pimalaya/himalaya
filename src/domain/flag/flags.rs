use serde::Serialize;
use std::{collections::HashSet, ops};

use crate::Flag;

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
