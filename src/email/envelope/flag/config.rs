use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::backend::BackendKind;

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct FlagConfig {
    #[cfg(feature = "flag-add")]
    pub add: Option<FlagAddConfig>,
    #[cfg(feature = "flag-set")]
    pub set: Option<FlagSetConfig>,
    #[cfg(feature = "flag-remove")]
    pub remove: Option<FlagRemoveConfig>,
}

impl FlagConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        #[allow(unused_mut)]
        let mut kinds = HashSet::default();

        #[cfg(feature = "flag-add")]
        if let Some(add) = &self.add {
            kinds.extend(add.get_used_backends());
        }

        #[cfg(feature = "flag-set")]
        if let Some(set) = &self.set {
            kinds.extend(set.get_used_backends());
        }

        #[cfg(feature = "flag-remove")]
        if let Some(remove) = &self.remove {
            kinds.extend(remove.get_used_backends());
        }

        kinds
    }
}

#[cfg(feature = "flag-add")]
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct FlagAddConfig {
    pub backend: Option<BackendKind>,
}

#[cfg(feature = "flag-add")]
impl FlagAddConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}

#[cfg(feature = "flag-set")]
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct FlagSetConfig {
    pub backend: Option<BackendKind>,
}

#[cfg(feature = "flag-set")]
impl FlagSetConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}

#[cfg(feature = "flag-remove")]
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct FlagRemoveConfig {
    pub backend: Option<BackendKind>,
}

#[cfg(feature = "flag-remove")]
impl FlagRemoveConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}
