use serde::Serialize;
use std::ops::{Deref, DerefMut};

use super::Mbox;

/// Represents the list of mailboxes.
#[derive(Debug, Default, Serialize)]
pub struct Mboxes {
    #[serde(rename = "response")]
    pub mboxes: Vec<Mbox>,
}

impl Deref for Mboxes {
    type Target = Vec<Mbox>;

    fn deref(&self) -> &Self::Target {
        &self.mboxes
    }
}

impl DerefMut for Mboxes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.mboxes
    }
}
