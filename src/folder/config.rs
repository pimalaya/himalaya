use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::backend::BackendKind;

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct FolderConfig {
    #[serde(alias = "aliases")]
    pub alias: Option<HashMap<String, String>>,

    pub add: Option<FolderAddConfig>,
    pub list: Option<FolderListConfig>,
    pub expunge: Option<FolderExpungeConfig>,
    pub purge: Option<FolderPurgeConfig>,
    pub delete: Option<FolderDeleteConfig>,
}

impl FolderConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(add) = &self.add {
            kinds.extend(add.get_used_backends());
        }

        if let Some(list) = &self.list {
            kinds.extend(list.get_used_backends());
        }

        if let Some(expunge) = &self.expunge {
            kinds.extend(expunge.get_used_backends());
        }

        if let Some(purge) = &self.purge {
            kinds.extend(purge.get_used_backends());
        }

        if let Some(delete) = &self.delete {
            kinds.extend(delete.get_used_backends());
        }

        kinds
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct FolderAddConfig {
    pub backend: Option<BackendKind>,
}

impl FolderAddConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct FolderListConfig {
    pub backend: Option<BackendKind>,

    #[serde(flatten)]
    pub remote: email::folder::list::config::FolderListConfig,
}

impl FolderListConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct FolderExpungeConfig {
    pub backend: Option<BackendKind>,
}

impl FolderExpungeConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct FolderPurgeConfig {
    pub backend: Option<BackendKind>,
}

impl FolderPurgeConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct FolderDeleteConfig {
    pub backend: Option<BackendKind>,
}

impl FolderDeleteConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}
