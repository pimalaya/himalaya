use ::serde::{Deserialize, Serialize};

use crate::backend::BackendKind;

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct FolderConfig {
    pub add: Option<FolderAddConfig>,
    pub list: Option<FolderListConfig>,
    pub expunge: Option<FolderExpungeConfig>,
    pub purge: Option<FolderPurgeConfig>,
    pub delete: Option<FolderDeleteConfig>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct FolderAddConfig {
    pub backend: Option<BackendKind>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct FolderListConfig {
    pub backend: Option<BackendKind>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct FolderExpungeConfig {
    pub backend: Option<BackendKind>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct FolderPurgeConfig {
    pub backend: Option<BackendKind>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct FolderDeleteConfig {
    pub backend: Option<BackendKind>,
}
