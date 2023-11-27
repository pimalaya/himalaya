use ::serde::{Deserialize, Serialize};

use crate::backend::BackendKind;

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct FlagConfig {
    pub add: Option<FlagAddConfig>,
    pub set: Option<FlagSetConfig>,
    pub remove: Option<FlagRemoveConfig>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct FlagAddConfig {
    pub backend: Option<BackendKind>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct FlagSetConfig {
    pub backend: Option<BackendKind>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct FlagRemoveConfig {
    pub backend: Option<BackendKind>,
}
