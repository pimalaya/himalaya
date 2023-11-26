//! Deserialized account config module.
//!
//! This module contains the raw deserialized representation of an
//! account in the accounts section of the user configuration file.

#[cfg(feature = "pgp")]
use email::account::PgpConfig;
#[cfg(feature = "imap-backend")]
use email::imap::ImapConfig;
#[cfg(feature = "smtp-sender")]
use email::smtp::SmtpConfig;
use email::{
    email::{EmailHooks, EmailTextPlainFormat},
    folder::sync::FolderSyncStrategy,
    maildir::MaildirConfig,
    sendmail::SendmailConfig,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

use crate::{
    backend::BackendKind, config::prelude::*, domain::config::FolderConfig,
    email::envelope::config::EnvelopeConfig, message::config::MessageConfig,
};

/// Represents all existing kind of account config.
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(tag = "backend", rename_all = "kebab-case")]
pub struct DeserializedAccountConfig {
    pub default: Option<bool>,

    pub email: String,
    pub display_name: Option<String>,
    pub signature_delim: Option<String>,
    pub signature: Option<String>,
    pub downloads_dir: Option<PathBuf>,

    pub folder_listing_page_size: Option<usize>,
    pub folder_aliases: Option<HashMap<String, String>>,

    pub email_listing_page_size: Option<usize>,
    pub email_listing_datetime_fmt: Option<String>,
    pub email_listing_datetime_local_tz: Option<bool>,
    pub email_reading_headers: Option<Vec<String>>,
    #[serde(default, with = "OptionEmailTextPlainFormatDef")]
    pub email_reading_format: Option<EmailTextPlainFormat>,
    pub email_writing_headers: Option<Vec<String>>,
    pub email_sending_save_copy: Option<bool>,
    #[serde(default, with = "OptionEmailHooksDef")]
    pub email_hooks: Option<EmailHooks>,

    pub sync: Option<bool>,
    pub sync_dir: Option<PathBuf>,
    #[serde(default, with = "OptionFolderSyncStrategyDef")]
    pub sync_folders_strategy: Option<FolderSyncStrategy>,

    pub backend: Option<BackendKind>,

    pub folder: Option<FolderConfig>,
    pub envelope: Option<EnvelopeConfig>,
    pub message: Option<MessageConfig>,

    #[cfg(feature = "imap-backend")]
    #[serde(default, with = "OptionImapConfigDef")]
    pub imap: Option<ImapConfig>,

    #[serde(default, with = "OptionMaildirConfigDef")]
    pub maildir: Option<MaildirConfig>,

    #[cfg(feature = "notmuch-backend")]
    #[serde(default, with = "OptionNotmuchConfigDef")]
    pub notmuch: Option<NotmuchConfig>,

    #[cfg(feature = "smtp-sender")]
    #[serde(default, with = "OptionSmtpConfigDef")]
    pub smtp: Option<SmtpConfig>,

    #[serde(default, with = "OptionSendmailConfigDef")]
    pub sendmail: Option<SendmailConfig>,

    #[cfg(feature = "pgp")]
    #[serde(default, with = "OptionPgpConfigDef")]
    pub pgp: Option<PgpConfig>,
}
