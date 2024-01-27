use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::backend::BackendKind;

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct FolderConfig {
    #[serde(alias = "aliases")]
    pub alias: Option<HashMap<String, String>>,

    #[cfg(any(feature = "account-sync", feature = "folder-add"))]
    pub add: Option<FolderAddConfig>,
    #[cfg(any(feature = "account-sync", feature = "folder-list"))]
    pub list: Option<FolderListConfig>,
    #[cfg(any(feature = "account-sync", feature = "folder-expunge"))]
    pub expunge: Option<FolderExpungeConfig>,
    #[cfg(feature = "folder-purge")]
    pub purge: Option<FolderPurgeConfig>,
    #[cfg(any(feature = "account-sync", feature = "folder-delete"))]
    pub delete: Option<FolderDeleteConfig>,
}

impl FolderConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        #[allow(unused_mut)]
        let mut kinds = HashSet::default();

        #[cfg(any(feature = "account-sync", feature = "folder-add"))]
        if let Some(add) = &self.add {
            kinds.extend(add.get_used_backends());
        }

        #[cfg(any(feature = "account-sync", feature = "folder-list"))]
        if let Some(list) = &self.list {
            kinds.extend(list.get_used_backends());
        }

        #[cfg(any(feature = "account-sync", feature = "folder-expunge"))]
        if let Some(expunge) = &self.expunge {
            kinds.extend(expunge.get_used_backends());
        }

        #[cfg(feature = "folder-purge")]
        if let Some(purge) = &self.purge {
            kinds.extend(purge.get_used_backends());
        }

        #[cfg(any(feature = "account-sync", feature = "folder-delete"))]
        if let Some(delete) = &self.delete {
            kinds.extend(delete.get_used_backends());
        }

        kinds
    }
}

#[cfg(any(feature = "account-sync", feature = "folder-add"))]
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct FolderAddConfig {
    pub backend: Option<BackendKind>,
}

#[cfg(any(feature = "account-sync", feature = "folder-add"))]
impl FolderAddConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}

#[cfg(any(feature = "account-sync", feature = "folder-list"))]
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct FolderListConfig {
    pub backend: Option<BackendKind>,

    #[serde(flatten)]
    pub remote: email::folder::list::config::FolderListConfig,
}

#[cfg(any(feature = "account-sync", feature = "folder-list"))]
impl FolderListConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}

#[cfg(any(feature = "account-sync", feature = "folder-expunge"))]
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct FolderExpungeConfig {
    pub backend: Option<BackendKind>,
}

#[cfg(any(feature = "account-sync", feature = "folder-expunge"))]
impl FolderExpungeConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}

#[cfg(feature = "folder-purge")]
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct FolderPurgeConfig {
    pub backend: Option<BackendKind>,
}

#[cfg(feature = "folder-purge")]
impl FolderPurgeConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}

#[cfg(any(feature = "account-sync", feature = "folder-delete"))]
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct FolderDeleteConfig {
    pub backend: Option<BackendKind>,
}

#[cfg(any(feature = "account-sync", feature = "folder-delete"))]
impl FolderDeleteConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}
