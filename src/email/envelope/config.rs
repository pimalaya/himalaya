use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::backend::BackendKind;

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct EnvelopeConfig {
    pub list: Option<ListEnvelopesConfig>,
    pub thread: Option<ThreadEnvelopesConfig>,
    pub get: Option<GetEnvelopeConfig>,
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
pub struct ListEnvelopesConfig {
    pub backend: Option<BackendKind>,

    #[serde(flatten)]
    pub remote: email::envelope::list::config::EnvelopeListConfig,
}

impl ListEnvelopesConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct ThreadEnvelopesConfig {
    pub backend: Option<BackendKind>,

    #[serde(flatten)]
    pub remote: email::envelope::thread::config::EnvelopeThreadConfig,
}

impl ThreadEnvelopesConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct GetEnvelopeConfig {
    pub backend: Option<BackendKind>,
}

impl GetEnvelopeConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}
