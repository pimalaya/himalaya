use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::backend::BackendKind;

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct EnvelopeConfig {
    pub list: Option<EnvelopeListConfig>,
    pub get: Option<EnvelopeGetConfig>,
}

impl EnvelopeConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(list) = &self.list {
            kinds.extend(list.get_used_backends());
        }

        if let Some(get) = &self.get {
            kinds.extend(get.get_used_backends());
        }

        kinds
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct EnvelopeListConfig {
    pub backend: Option<BackendKind>,
}

impl EnvelopeListConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct EnvelopeGetConfig {
    pub backend: Option<BackendKind>,
}

impl EnvelopeGetConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}
