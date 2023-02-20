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

impl From<himalaya_lib::Flags> for Flags {
    fn from(flags: himalaya_lib::Flags) -> Self {
        Flags(flags.iter().map(Flag::from).collect())
    }
}
