use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::backend::BackendKind;

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct EnvelopeConfig {
    #[cfg(feature = "envelope-list")]
    pub list: Option<ListEnvelopesConfig>,
    #[cfg(feature = "envelope-watch")]
    pub watch: Option<WatchEnvelopesConfig>,
    #[cfg(feature = "envelope-get")]
    pub get: Option<GetEnvelopeConfig>,
}

impl EnvelopeConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        #[allow(unused_mut)]
        let mut kinds = HashSet::default();

        #[cfg(feature = "envelope-list")]
        if let Some(list) = &self.list {
            kinds.extend(list.get_used_backends());
        }

        #[cfg(feature = "envelope-watch")]
        if let Some(watch) = &self.watch {
            kinds.extend(watch.get_used_backends());
        }

        #[cfg(feature = "envelope-get")]
        if let Some(get) = &self.get {
            kinds.extend(get.get_used_backends());
        }

        kinds
    }
}

#[cfg(feature = "envelope-list")]
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct ListEnvelopesConfig {
    pub backend: Option<BackendKind>,

    #[serde(flatten)]
    pub remote: email::envelope::list::config::EnvelopeListConfig,
}

#[cfg(feature = "envelope-list")]
impl ListEnvelopesConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}

#[cfg(feature = "envelope-watch")]
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct WatchEnvelopesConfig {
    pub backend: Option<BackendKind>,

    #[serde(flatten)]
    pub remote: email::envelope::watch::config::WatchEnvelopeConfig,
}

#[cfg(feature = "envelope-watch")]
impl WatchEnvelopesConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}

#[cfg(feature = "envelope-get")]
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct GetEnvelopeConfig {
    pub backend: Option<BackendKind>,
}

#[cfg(feature = "envelope-get")]
impl GetEnvelopeConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}
