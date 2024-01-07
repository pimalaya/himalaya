use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::backend::BackendKind;

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct MessageConfig {
    #[cfg(any(feature = "message-add", feature = "message-write"))]
    pub write: Option<MessageAddConfig>,
    #[cfg(any(feature = "message-send", feature = "template-send"))]
    pub send: Option<MessageSendConfig>,
    #[cfg(feature = "message-peek")]
    pub peek: Option<MessagePeekConfig>,
    #[cfg(any(feature = "message-get", feature = "message-read"))]
    pub read: Option<MessageGetConfig>,
    #[cfg(feature = "message-copy")]
    pub copy: Option<MessageCopyConfig>,
    #[cfg(feature = "message-move")]
    #[serde(rename = "move")]
    pub move_: Option<MessageMoveConfig>,
}

impl MessageConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        #[allow(unused_mut)]
        let mut kinds = HashSet::default();

        #[cfg(any(feature = "message-add", feature = "message-write"))]
        if let Some(add) = &self.write {
            kinds.extend(add.get_used_backends());
        }

        #[cfg(any(feature = "message-send", feature = "template-send"))]
        if let Some(send) = &self.send {
            kinds.extend(send.get_used_backends());
        }

        #[cfg(feature = "message-peek")]
        if let Some(peek) = &self.peek {
            kinds.extend(peek.get_used_backends());
        }

        #[cfg(any(feature = "message-get", feature = "message-read"))]
        if let Some(get) = &self.read {
            kinds.extend(get.get_used_backends());
        }

        #[cfg(feature = "message-copy")]
        if let Some(copy) = &self.copy {
            kinds.extend(copy.get_used_backends());
        }

        #[cfg(feature = "message-move")]
        if let Some(move_) = &self.move_ {
            kinds.extend(move_.get_used_backends());
        }

        kinds
    }
}

#[cfg(any(feature = "message-add", feature = "message-write"))]
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct MessageAddConfig {
    pub backend: Option<BackendKind>,

    #[serde(flatten)]
    pub remote: email::message::add::config::MessageWriteConfig,
}

#[cfg(any(feature = "message-add", feature = "message-write"))]
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

#[cfg(feature = "message-peek")]
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct MessagePeekConfig {
    pub backend: Option<BackendKind>,
}

#[cfg(feature = "message-peek")]
impl MessagePeekConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}

#[cfg(any(feature = "message-get", feature = "message-read"))]
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct MessageGetConfig {
    pub backend: Option<BackendKind>,

    #[serde(flatten)]
    pub remote: email::message::get::config::MessageReadConfig,
}

#[cfg(any(feature = "message-get", feature = "message-read"))]
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

#[cfg(feature = "message-move")]
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct MessageMoveConfig {
    pub backend: Option<BackendKind>,
}

#[cfg(feature = "message-move")]
impl MessageMoveConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}
