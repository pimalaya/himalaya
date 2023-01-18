//! Deserialized account config module.
//!
//! This module contains the raw deserialized representation of an
//! account in the accounts section of the user configuration file.

use himalaya_lib::{AccountConfig, BackendConfig, EmailHooks, EmailSender, EmailTextPlainFormat};

#[cfg(feature = "imap-backend")]
use himalaya_lib::ImapConfig;

#[cfg(feature = "maildir-backend")]
use himalaya_lib::MaildirConfig;

#[cfg(feature = "notmuch-backend")]
use himalaya_lib::NotmuchConfig;

use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

use crate::config::{prelude::*, DeserializedConfig};

/// Represents all existing kind of account config.
#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(tag = "backend", rename_all = "snake_case")]
pub enum DeserializedAccountConfig {
    None(DeserializedBaseAccountConfig),
    #[cfg(feature = "imap-backend")]
    Imap(DeserializedImapAccountConfig),
    #[cfg(feature = "maildir-backend")]
    Maildir(DeserializedMaildirAccountConfig),
    #[cfg(feature = "notmuch-backend")]
    Notmuch(DeserializedNotmuchAccountConfig),
}

impl DeserializedAccountConfig {
    pub fn to_configs(&self, global_config: &DeserializedConfig) -> (AccountConfig, BackendConfig) {
        match self {
            DeserializedAccountConfig::None(config) => {
                (config.to_account_config(global_config), BackendConfig::None)
            }
            #[cfg(feature = "imap-backend")]
            DeserializedAccountConfig::Imap(config) => (
                config.base.to_account_config(global_config),
                BackendConfig::Imap(&config.backend),
            ),
            #[cfg(feature = "maildir-backend")]
            DeserializedAccountConfig::Maildir(config) => (
                config.base.to_account_config(global_config),
                BackendConfig::Maildir(&config.backend),
            ),
            #[cfg(feature = "notmuch-backend")]
            DeserializedAccountConfig::Notmuch(config) => (
                config.base.to_account_config(global_config),
                BackendConfig::Notmuch(&config.backend),
            ),
        }
    }

    pub fn is_default(&self) -> bool {
        match self {
            DeserializedAccountConfig::None(config) => config.default.unwrap_or_default(),
            #[cfg(feature = "imap-backend")]
            DeserializedAccountConfig::Imap(config) => config.base.default.unwrap_or_default(),
            #[cfg(feature = "maildir-backend")]
            DeserializedAccountConfig::Maildir(config) => config.base.default.unwrap_or_default(),
            #[cfg(feature = "notmuch-backend")]
            DeserializedAccountConfig::Notmuch(config) => config.base.default.unwrap_or_default(),
        }
    }
}

#[derive(Default, Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct DeserializedBaseAccountConfig {
    pub email: String,
    pub default: Option<bool>,
    pub display_name: Option<String>,
    pub signature_delim: Option<String>,
    pub signature: Option<String>,
    pub downloads_dir: Option<PathBuf>,

    pub folder_listing_page_size: Option<usize>,
    pub folder_aliases: Option<HashMap<String, String>>,

    pub email_listing_page_size: Option<usize>,
    pub email_reading_headers: Option<Vec<String>>,
    #[serde(with = "EmailTextPlainFormatOptionDef", skip_serializing_if = "Option::is_none")]
    pub email_reading_format: Option<EmailTextPlainFormat>,
    pub email_reading_verify_cmd: Option<String>,
    pub email_reading_decrypt_cmd: Option<String>,
    pub email_writing_headers: Option<Vec<String>>,
    pub email_writing_sign_cmd: Option<String>,
    pub email_writing_encrypt_cmd: Option<String>,
    #[serde(flatten, with = "EmailSenderDef")]
    pub email_sender: EmailSender,
    #[serde(with = "EmailHooksOptionDef", skip_serializing_if = "Option::is_none")]
    pub email_hooks: Option<EmailHooks>,
}

impl DeserializedBaseAccountConfig {
    pub fn to_account_config(&self, config: &DeserializedConfig) -> AccountConfig {
        let mut folder_aliases = config
            .folder_aliases
            .as_ref()
            .map(ToOwned::to_owned)
            .unwrap_or_default();
        folder_aliases.extend(
            self.folder_aliases
                .as_ref()
                .map(ToOwned::to_owned)
                .unwrap_or_default(),
        );

        AccountConfig {
            email: self.email.to_owned(),
            display_name: self
                .display_name
                .as_ref()
                .map(ToOwned::to_owned)
                .or_else(|| config.display_name.as_ref().map(ToOwned::to_owned)),
            signature_delim: self
                .signature_delim
                .as_ref()
                .map(ToOwned::to_owned)
                .or_else(|| config.signature_delim.as_ref().map(ToOwned::to_owned)),
            signature: self
                .signature
                .as_ref()
                .map(ToOwned::to_owned)
                .or_else(|| config.signature.as_ref().map(ToOwned::to_owned)),
            downloads_dir: self
                .downloads_dir
                .as_ref()
                .map(ToOwned::to_owned)
                .or_else(|| config.downloads_dir.as_ref().map(ToOwned::to_owned)),
            folder_listing_page_size: self
                .folder_listing_page_size
                .or_else(|| config.folder_listing_page_size),
            folder_aliases,
            email_listing_page_size: self
                .email_listing_page_size
                .or_else(|| config.email_listing_page_size),
            email_reading_headers: self
                .email_reading_headers
                .as_ref()
                .map(ToOwned::to_owned)
                .or_else(|| config.email_reading_headers.as_ref().map(ToOwned::to_owned)),
            email_reading_format: self
                .email_reading_format
                .as_ref()
                .map(ToOwned::to_owned)
                .or_else(|| config.email_reading_format.as_ref().map(ToOwned::to_owned))
                .unwrap_or_default(),
            email_reading_verify_cmd: self
                .email_reading_verify_cmd
                .as_ref()
                .map(ToOwned::to_owned)
                .or_else(|| {
                    config
                        .email_reading_verify_cmd
                        .as_ref()
                        .map(ToOwned::to_owned)
                }),
            email_reading_decrypt_cmd: self
                .email_reading_decrypt_cmd
                .as_ref()
                .map(ToOwned::to_owned)
                .or_else(|| {
                    config
                        .email_reading_decrypt_cmd
                        .as_ref()
                        .map(ToOwned::to_owned)
                }),
            email_writing_sign_cmd: self
                .email_writing_sign_cmd
                .as_ref()
                .map(ToOwned::to_owned)
                .or_else(|| {
                    config
                        .email_writing_sign_cmd
                        .as_ref()
                        .map(ToOwned::to_owned)
                }),
            email_writing_encrypt_cmd: self
                .email_writing_encrypt_cmd
                .as_ref()
                .map(ToOwned::to_owned)
                .or_else(|| {
                    config
                        .email_writing_encrypt_cmd
                        .as_ref()
                        .map(ToOwned::to_owned)
                }),
            email_writing_headers: self
                .email_writing_headers
                .as_ref()
                .map(ToOwned::to_owned)
                .or_else(|| config.email_writing_headers.as_ref().map(ToOwned::to_owned)),
            email_sender: self.email_sender.to_owned(),
            email_hooks: EmailHooks {
                pre_send: self
                    .email_hooks
                    .as_ref()
                    .map(ToOwned::to_owned)
                    .map(|hook| hook.pre_send)
                    .or_else(|| {
                        config
                            .email_hooks
                            .as_ref()
                            .map(|hook| hook.pre_send.as_ref().map(ToOwned::to_owned))
                    })
                    .unwrap_or_default(),
            },
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[cfg(feature = "imap-backend")]
pub struct DeserializedImapAccountConfig {
    #[serde(flatten)]
    pub base: DeserializedBaseAccountConfig,
    #[serde(flatten, with = "ImapConfigDef")]
    pub backend: ImapConfig,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[cfg(feature = "maildir-backend")]
pub struct DeserializedMaildirAccountConfig {
    #[serde(flatten)]
    pub base: DeserializedBaseAccountConfig,
    #[serde(flatten, with = "MaildirConfigDef")]
    pub backend: MaildirConfig,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[cfg(feature = "notmuch-backend")]
pub struct DeserializedNotmuchAccountConfig {
    #[serde(flatten)]
    pub base: DeserializedBaseAccountConfig,
    #[serde(flatten, with = "NotmuchConfigDef")]
    pub backend: NotmuchConfig,
}
