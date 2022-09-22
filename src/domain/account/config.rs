// himalaya-lib, a Rust library for email management.
// Copyright (C) 2022  soywod <clement.douin@posteo.net>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! Deserialized account config module.
//!
//! This module contains the raw deserialized representation of an
//! account in the accounts section of the user configuration file.

use himalaya_lib::{
    AccountConfig, BackendConfig, EmailHooks, EmailSender, EmailTextPlainFormat, ImapConfig,
    MaildirConfig, NotmuchConfig,
};
use serde::Deserialize;
use std::{collections::HashMap, path::PathBuf};

use crate::config::{prelude::*, DeserializedConfig};

/// Represents all existing kind of account config.
#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
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

#[derive(Default, Debug, Clone, Eq, PartialEq, Deserialize)]
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
    #[serde(default, with = "email_text_plain_format")]
    pub email_reading_format: Option<EmailTextPlainFormat>,
    pub email_reading_decrypt_cmd: Option<String>,
    pub email_writing_encrypt_cmd: Option<String>,
    #[serde(flatten, with = "EmailSenderDef")]
    pub email_sender: EmailSender,
    #[serde(default, with = "email_hooks")]
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

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[cfg(feature = "imap-backend")]
pub struct DeserializedImapAccountConfig {
    #[serde(flatten)]
    pub base: DeserializedBaseAccountConfig,
    #[serde(flatten, with = "ImapConfigDef")]
    pub backend: ImapConfig,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[cfg(feature = "maildir-backend")]
pub struct DeserializedMaildirAccountConfig {
    #[serde(flatten)]
    pub base: DeserializedBaseAccountConfig,
    #[serde(flatten, with = "MaildirConfigDef")]
    pub backend: MaildirConfig,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[cfg(feature = "notmuch-backend")]
pub struct DeserializedNotmuchAccountConfig {
    #[serde(flatten)]
    pub base: DeserializedBaseAccountConfig,
    #[serde(flatten, with = "NotmuchConfigDef")]
    pub backend: NotmuchConfig,
}
