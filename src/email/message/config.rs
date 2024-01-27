use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::backend::BackendKind;

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct MessageConfig {
    #[cfg(any(feature = "account-sync", feature = "message-add"))]
    pub write: Option<MessageAddConfig>,
    #[cfg(any(feature = "message-send", feature = "template-send"))]
    pub send: Option<MessageSendConfig>,
    #[cfg(feature = "account-sync")]
    pub peek: Option<MessagePeekConfig>,
    #[cfg(any(feature = "account-sync", feature = "message-get"))]
    pub read: Option<MessageGetConfig>,
    #[cfg(feature = "message-copy")]
    pub copy: Option<MessageCopyConfig>,
    #[cfg(any(feature = "account-sync", feature = "message-move"))]
    pub r#move: Option<MessageMoveConfig>,
    #[cfg(feature = "message-delete")]
    pub delete: Option<MessageDeleteConfig>,
}

impl MessageConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        #[allow(unused_mut)]
        let mut kinds = HashSet::default();

        #[cfg(any(feature = "account-sync", feature = "message-add"))]
        if let Some(add) = &self.write {
            kinds.extend(add.get_used_backends());
        }

        #[cfg(any(feature = "message-send", feature = "template-send"))]
        if let Some(send) = &self.send {
            kinds.extend(send.get_used_backends());
        }

        #[cfg(feature = "account-sync")]
        if let Some(peek) = &self.peek {
            kinds.extend(peek.get_used_backends());
        }

        #[cfg(any(feature = "account-sync", feature = "message-get"))]
        if let Some(get) = &self.read {
            kinds.extend(get.get_used_backends());
        }

        #[cfg(feature = "message-copy")]
        if let Some(copy) = &self.copy {
            kinds.extend(copy.get_used_backends());
        }

        #[cfg(any(feature = "account-sync", feature = "message-move"))]
        if let Some(move_) = &self.r#move {
            kinds.extend(move_.get_used_backends());
        }

        kinds
    }
}

#[cfg(any(feature = "account-sync", feature = "message-add"))]
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct MessageAddConfig {
    pub backend: Option<BackendKind>,

    #[serde(flatten)]
    pub remote: email::message::add::config::MessageWriteConfig,
}

#[cfg(any(feature = "account-sync", feature = "message-add"))]
impl MessageAddConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}

#[cfg(any(feature = "message-send", feature = "template-send"))]
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct MessageSendConfig {
    pub backend: Option<BackendKind>,

    #[serde(flatten)]
    pub remote: email::message::send::config::MessageSendConfig,
}

#[cfg(any(feature = "message-send", feature = "template-send"))]
impl MessageSendConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}

#[cfg(any(feature = "account-sync", feature = "message-peek"))]
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct MessagePeekConfig {
    pub backend: Option<BackendKind>,
}

#[cfg(any(feature = "account-sync", feature = "message-peek"))]
impl MessagePeekConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}

#[cfg(any(feature = "account-sync", feature = "message-get"))]
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct MessageGetConfig {
    pub backend: Option<BackendKind>,

    #[serde(flatten)]
    pub remote: email::message::get::config::MessageReadConfig,
}

#[cfg(any(feature = "account-sync", feature = "message-get"))]
impl MessageGetConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}

#[cfg(feature = "message-copy")]
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct MessageCopyConfig {
    pub backend: Option<BackendKind>,
}

#[cfg(feature = "message-copy")]
impl MessageCopyConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}

#[cfg(any(feature = "account-sync", feature = "message-move"))]
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct MessageMoveConfig {
    pub backend: Option<BackendKind>,
}

#[cfg(any(feature = "account-sync", feature = "message-move"))]
impl MessageMoveConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}

#[cfg(feature = "message-delete")]
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct MessageDeleteConfig {
    pub backend: Option<BackendKind>,
}

#[cfg(feature = "message-delete")]
impl MessageDeleteConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}
