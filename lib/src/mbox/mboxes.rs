//! Mailboxes module.
//!
//! This module contains the representation of the mailboxes.

use serde::Serialize;
use std::ops;

use super::Mbox;

/// Represents the list of mailboxes.
#[derive(Debug, Default, Serialize)]
pub struct Mboxes {
    #[serde(rename = "response")]
    pub mboxes: Vec<Mbox>,
}

impl ops::Deref for Mboxes {
    type Target = Vec<Mbox>;

    fn deref(&self) -> &Self::Target {
        &self.mboxes
    }
}

impl ops::DerefMut for Mboxes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.mboxes
    }
}
