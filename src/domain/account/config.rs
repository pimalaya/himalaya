//! Deserialized account config module.
//!
//! This module contains the raw deserialized representation of an
//! account in the accounts section of the user configuration file.

use pimalaya_email::{
    folder::sync::Strategy as SyncFoldersStrategy, AccountConfig, BackendConfig, EmailHooks,
    EmailTextPlainFormat, SenderConfig,
};
use pimalaya_process::Cmd;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

use crate::config::{prelude::*, DeserializedConfig};

/// Represents all existing kind of account config.
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(tag = "backend", rename_all = "kebab-case")]
pub struct DeserializedAccountConfig {
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
    #[serde(
        default,
        with = "OptionCmdDef",
        skip_serializing_if = "Option::is_none"
    )]
    pub email_reading_verify_cmd: Option<Cmd>,
    #[serde(
        default,
        with = "OptionCmdDef",
        skip_serializing_if = "Option::is_none"
    )]
    pub email_reading_decrypt_cmd: Option<Cmd>,
    pub email_writing_headers: Option<Vec<String>>,
    #[serde(
        default,
        with = "OptionCmdDef",
        skip_serializing_if = "Option::is_none"
    )]
    pub email_writing_sign_cmd: Option<Cmd>,
    #[serde(
        default,
        with = "OptionCmdDef",
        skip_serializing_if = "Option::is_none"
    )]
    pub email_writing_encrypt_cmd: Option<Cmd>,
    pub email_sending_save_copy: Option<bool>,
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

    #[serde(flatten, with = "BackendConfigDef")]
    pub backend: BackendConfig,
    #[serde(flatten, with = "SenderConfigDef")]
    pub sender: SenderConfig,
}

impl DeserializedAccountConfig {
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
            email_sending_save_copy: self.email_sending_save_copy.unwrap_or(true),
            email_hooks: EmailHooks {
                pre_send: self.email_hooks.pre_send.clone(),
            },
            sync: self.sync,
            sync_dir: self.sync_dir.clone(),
            sync_folders_strategy: self.sync_folders_strategy.clone(),

            backend: self.backend.clone(),
            sender: self.sender.clone(),
        }
    }
}
