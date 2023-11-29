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
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use crate::{
    backend::BackendKind,
    config::prelude::*,
    domain::config::FolderConfig,
    email::envelope::{config::EnvelopeConfig, flag::config::FlagConfig},
    message::config::MessageConfig,
};

/// Represents all existing kind of account config.
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(tag = "backend", rename_all = "kebab-case")]
pub struct TomlAccountConfig {
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
    pub flag: Option<FlagConfig>,
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

impl TomlAccountConfig {
    pub fn add_folder_kind(&self) -> Option<&BackendKind> {
        self.folder
            .as_ref()
            .and_then(|folder| folder.add.as_ref())
            .and_then(|add| add.backend.as_ref())
            .or_else(|| self.backend.as_ref())
    }

    pub fn list_folders_kind(&self) -> Option<&BackendKind> {
        self.folder
            .as_ref()
            .and_then(|folder| folder.list.as_ref())
            .and_then(|list| list.backend.as_ref())
            .or_else(|| self.backend.as_ref())
    }

    pub fn expunge_folder_kind(&self) -> Option<&BackendKind> {
        self.folder
            .as_ref()
            .and_then(|folder| folder.expunge.as_ref())
            .and_then(|expunge| expunge.backend.as_ref())
            .or_else(|| self.backend.as_ref())
    }

    pub fn purge_folder_kind(&self) -> Option<&BackendKind> {
        self.folder
            .as_ref()
            .and_then(|folder| folder.purge.as_ref())
            .and_then(|purge| purge.backend.as_ref())
            .or_else(|| self.backend.as_ref())
    }

    pub fn delete_folder_kind(&self) -> Option<&BackendKind> {
        self.folder
            .as_ref()
            .and_then(|folder| folder.delete.as_ref())
            .and_then(|delete| delete.backend.as_ref())
            .or_else(|| self.backend.as_ref())
    }

    pub fn get_envelope_kind(&self) -> Option<&BackendKind> {
        self.envelope
            .as_ref()
            .and_then(|envelope| envelope.get.as_ref())
            .and_then(|get| get.backend.as_ref())
            .or_else(|| self.backend.as_ref())
    }

    pub fn list_envelopes_kind(&self) -> Option<&BackendKind> {
        self.envelope
            .as_ref()
            .and_then(|envelope| envelope.list.as_ref())
            .and_then(|list| list.backend.as_ref())
            .or_else(|| self.backend.as_ref())
    }

    pub fn add_flags_kind(&self) -> Option<&BackendKind> {
        self.flag
            .as_ref()
            .and_then(|flag| flag.add.as_ref())
            .and_then(|add| add.backend.as_ref())
            .or_else(|| self.backend.as_ref())
    }

    pub fn set_flags_kind(&self) -> Option<&BackendKind> {
        self.flag
            .as_ref()
            .and_then(|flag| flag.set.as_ref())
            .and_then(|set| set.backend.as_ref())
            .or_else(|| self.backend.as_ref())
    }

    pub fn remove_flags_kind(&self) -> Option<&BackendKind> {
        self.flag
            .as_ref()
            .and_then(|flag| flag.remove.as_ref())
            .and_then(|remove| remove.backend.as_ref())
            .or_else(|| self.backend.as_ref())
    }

    pub fn add_raw_message_kind(&self) -> Option<&BackendKind> {
        self.message
            .as_ref()
            .and_then(|msg| msg.add.as_ref())
            .and_then(|add| add.backend.as_ref())
            .or_else(|| self.backend.as_ref())
    }

    pub fn peek_messages_kind(&self) -> Option<&BackendKind> {
        self.message
            .as_ref()
            .and_then(|message| message.peek.as_ref())
            .and_then(|peek| peek.backend.as_ref())
            .or_else(|| self.backend.as_ref())
    }

    pub fn get_messages_kind(&self) -> Option<&BackendKind> {
        self.message
            .as_ref()
            .and_then(|message| message.get.as_ref())
            .and_then(|get| get.backend.as_ref())
            .or_else(|| self.backend.as_ref())
    }

    pub fn copy_messages_kind(&self) -> Option<&BackendKind> {
        self.message
            .as_ref()
            .and_then(|message| message.copy.as_ref())
            .and_then(|copy| copy.backend.as_ref())
            .or_else(|| self.backend.as_ref())
    }

    pub fn move_messages_kind(&self) -> Option<&BackendKind> {
        self.message
            .as_ref()
            .and_then(|message| message.move_.as_ref())
            .and_then(|move_| move_.backend.as_ref())
            .or_else(|| self.backend.as_ref())
    }

    pub fn delete_messages_kind(&self) -> Option<&BackendKind> {
        self.flag
            .as_ref()
            .and_then(|flag| flag.remove.as_ref())
            .and_then(|remove| remove.backend.as_ref())
            .or_else(|| self.backend.as_ref())
    }

    pub fn send_raw_message_kind(&self) -> Option<&BackendKind> {
        self.message
            .as_ref()
            .and_then(|msg| msg.send.as_ref())
            .and_then(|send| send.backend.as_ref())
            .or_else(|| self.backend.as_ref())
    }

    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut used_backends = HashSet::default();

        if let Some(ref kind) = self.backend {
            used_backends.insert(kind);
        }

        if let Some(ref folder) = self.folder {
            used_backends.extend(folder.get_used_backends());
        }

        if let Some(ref envelope) = self.envelope {
            used_backends.extend(envelope.get_used_backends());
        }

        if let Some(ref flag) = self.flag {
            used_backends.extend(flag.get_used_backends());
        }

        if let Some(ref msg) = self.message {
            used_backends.extend(msg.get_used_backends());
        }

        used_backends
    }
}
