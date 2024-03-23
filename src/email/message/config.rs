use email::message::delete::config::DeleteMessageStyle;
#[cfg(feature = "account-sync")]
use email::message::sync::config::MessageSyncConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::backend::BackendKind;

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct MessageConfig {
    pub write: Option<MessageAddConfig>,
    pub send: Option<MessageSendConfig>,
    pub peek: Option<MessagePeekConfig>,
    pub read: Option<MessageGetConfig>,
    pub copy: Option<MessageCopyConfig>,
    pub r#move: Option<MessageMoveConfig>,
    pub delete: Option<DeleteMessageConfig>,
    #[cfg(feature = "account-sync")]
    pub sync: Option<MessageSyncConfig>,
}

impl MessageConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(add) = &self.write {
            kinds.extend(add.get_used_backends());
        }

        if let Some(send) = &self.send {
            kinds.extend(send.get_used_backends());
        }

        if let Some(peek) = &self.peek {
            kinds.extend(peek.get_used_backends());
        }

        if let Some(get) = &self.read {
            kinds.extend(get.get_used_backends());
        }

        if let Some(copy) = &self.copy {
            kinds.extend(copy.get_used_backends());
        }

        if let Some(move_) = &self.r#move {
            kinds.extend(move_.get_used_backends());
        }

        kinds
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct MessageAddConfig {
    pub backend: Option<BackendKind>,

    #[serde(flatten)]
    pub remote: email::message::add::config::MessageWriteConfig,
}

impl MessageAddConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct MessageSendConfig {
    pub backend: Option<BackendKind>,

    #[serde(flatten)]
    pub remote: email::message::send::config::MessageSendConfig,
}

impl MessageSendConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct MessagePeekConfig {
    pub backend: Option<BackendKind>,
}

impl MessagePeekConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct MessageGetConfig {
    pub backend: Option<BackendKind>,

    #[serde(flatten)]
    pub remote: email::message::get::config::MessageReadConfig,
}

impl MessageGetConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct MessageCopyConfig {
    pub backend: Option<BackendKind>,
}

impl MessageCopyConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct MessageMoveConfig {
    pub backend: Option<BackendKind>,
}

impl MessageMoveConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct DeleteMessageConfig {
    pub backend: Option<BackendKind>,
    pub style: Option<DeleteMessageStyle>,
}

impl From<DeleteMessageConfig> for email::message::delete::config::DeleteMessageConfig {
    fn from(config: DeleteMessageConfig) -> Self {
        Self {
            style: config.style,
        }
    }
}

impl DeleteMessageConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}
