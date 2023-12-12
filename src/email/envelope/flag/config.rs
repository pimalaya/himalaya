use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::backend::BackendKind;

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct FlagConfig {
    pub add: Option<FlagAddConfig>,
    pub set: Option<FlagSetConfig>,
    pub remove: Option<FlagRemoveConfig>,
}

impl FlagConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(add) = &self.add {
            kinds.extend(add.get_used_backends());
        }

        if let Some(set) = &self.set {
            kinds.extend(set.get_used_backends());
        }

        if let Some(remove) = &self.remove {
            kinds.extend(remove.get_used_backends());
        }

        kinds
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct FlagAddConfig {
    pub backend: Option<BackendKind>,
}

impl FlagAddConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct FlagSetConfig {
    pub backend: Option<BackendKind>,
}

impl FlagSetConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct FlagRemoveConfig {
    pub backend: Option<BackendKind>,
}

impl FlagRemoveConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}
