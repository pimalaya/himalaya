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
    #[serde(rename = "move")]
    pub move_: Option<MessageMoveConfig>,
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

        if let Some(move_) = &self.move_ {
            kinds.extend(move_.get_used_backends());
        }

        kinds
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct MessageAddConfig {
    pub backend: Option<BackendKind>,

    #[serde(flatten)]
    pub remote: email::message::add_raw::config::MessageWriteConfig,
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
    pub remote: email::message::send_raw::config::MessageSendConfig,
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
