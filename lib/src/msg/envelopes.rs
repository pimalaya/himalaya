use serde::Serialize;
use std::ops;

use super::Envelope;

/// Represents the list of envelopes.
#[derive(Debug, Default, Serialize)]
pub struct Envelopes {
    #[serde(rename = "response")]
    pub envelopes: Vec<Envelope>,
}

impl ops::Deref for Envelopes {
    type Target = Vec<Envelope>;

    fn deref(&self) -> &Self::Target {
        &self.envelopes
    }
}

impl ops::DerefMut for Envelopes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.envelopes
    }
}
