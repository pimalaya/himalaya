//! Deserialized account config module.
//!
//! This module contains the raw deserialized representation of an
//! account in the accounts section of the user configuration file.

use himalaya_lib::{
    folder::sync::Strategy as SyncFoldersStrategy, AccountConfig, BackendConfig, EmailHooks,
    EmailSender, EmailTextPlainFormat, MaildirConfig,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

#[cfg(feature = "imap-backend")]
use himalaya_lib::ImapConfig;

#[cfg(feature = "notmuch-backend")]
use himalaya_lib::NotmuchConfig;

use crate::config::{prelude::*, DeserializedConfig};

/// Represents all existing kind of account config.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(tag = "backend", rename_all = "kebab-case")]
pub enum DeserializedAccountConfig {
    None(DeserializedBaseAccountConfig),
    Maildir(DeserializedMaildirAccountConfig),
    #[cfg(feature = "imap-backend")]
    Imap(DeserializedImapAccountConfig),
    #[cfg(feature = "notmuch-backend")]
    Notmuch(DeserializedNotmuchAccountConfig),
}

impl DeserializedAccountConfig {
    pub fn to_configs(
        &self,
        name: String,
        global_config: &DeserializedConfig,
    ) -> (AccountConfig, BackendConfig) {
        match self {
            DeserializedAccountConfig::None(config) => (
                config.to_account_config(name, global_config),
                BackendConfig::None,
            ),
            DeserializedAccountConfig::Maildir(config) => (
                config.base.to_account_config(name, global_config),
                BackendConfig::Maildir(config.backend.clone()),
            ),
            #[cfg(feature = "imap-backend")]
            DeserializedAccountConfig::Imap(config) => (
                config.base.to_account_config(name, global_config),
                BackendConfig::Imap(config.backend.clone()),
            ),
            #[cfg(feature = "notmuch-backend")]
            DeserializedAccountConfig::Notmuch(config) => (
                config.base.to_account_config(name, global_config),
                BackendConfig::Notmuch(config.backend.clone()),
            ),
        }
    }

    pub fn is_default(&self) -> bool {
        match self {
            DeserializedAccountConfig::None(config) => config.default.unwrap_or_default(),
            DeserializedAccountConfig::Maildir(config) => config.base.default.unwrap_or_default(),
            #[cfg(feature = "imap-backend")]
            DeserializedAccountConfig::Imap(config) => config.base.default.unwrap_or_default(),
            #[cfg(feature = "notmuch-backend")]
            DeserializedAccountConfig::Notmuch(config) => config.base.default.unwrap_or_default(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
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
    #[serde(default, with = "EmailTextPlainFormatDef")]
    pub email_reading_format: EmailTextPlainFormat,
    pub email_reading_verify_cmd: Option<String>,
    pub email_reading_decrypt_cmd: Option<String>,
    pub email_writing_headers: Option<Vec<String>>,
    pub email_writing_sign_cmd: Option<String>,
    pub email_writing_encrypt_cmd: Option<String>,
    #[serde(flatten, with = "EmailSenderDef")]
    pub email_sender: EmailSender,
    #[serde(
        default,
        with = "EmailHooksDef",
        skip_serializing_if = "EmailHooks::is_empty"
    )]
    pub email_hooks: EmailHooks,

    #[serde(default)]
    pub sync: bool,
    pub sync_dir: Option<PathBuf>,
    #[serde(default, with = "SyncFoldersStrategyDef")]
    pub sync_folders_strategy: SyncFoldersStrategy,
}

impl DeserializedBaseAccountConfig {
    pub fn to_account_config(&self, name: String, config: &DeserializedConfig) -> AccountConfig {
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
            name,
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
            email_reading_format: self.email_reading_format.clone(),
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
                pre_send: self.email_hooks.pre_send.clone(),
            },
            sync: self.sync,
            sync_dir: self.sync_dir.clone(),
            sync_folders_strategy: self.sync_folders_strategy.clone(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg(feature = "imap-backend")]
pub struct DeserializedImapAccountConfig {
    #[serde(flatten)]
    pub base: DeserializedBaseAccountConfig,
    #[serde(flatten, with = "ImapConfigDef")]
    pub backend: ImapConfig,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct DeserializedMaildirAccountConfig {
    #[serde(flatten)]
    pub base: DeserializedBaseAccountConfig,
    #[serde(flatten, with = "MaildirConfigDef")]
    pub backend: MaildirConfig,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg(feature = "notmuch-backend")]
pub struct DeserializedNotmuchAccountConfig {
    #[serde(flatten)]
    pub base: DeserializedBaseAccountConfig,
    #[serde(flatten, with = "NotmuchConfigDef")]
    pub backend: NotmuchConfig,
}
