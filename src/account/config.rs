//! Deserialized account config module.
//!
//! This module contains the raw deserialized representation of an
//! account in the accounts section of the user configuration file.

#[cfg(feature = "pgp")]
use email::account::config::pgp::PgpConfig;
#[cfg(feature = "account-sync")]
use email::account::sync::config::SyncConfig;
#[cfg(feature = "imap")]
use email::imap::config::ImapConfig;
#[cfg(feature = "maildir")]
use email::maildir::config::MaildirConfig;
#[cfg(feature = "sendmail")]
use email::sendmail::config::SendmailConfig;
#[cfg(feature = "smtp")]
use email::smtp::config::SmtpConfig;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::PathBuf};

use crate::{
    backend::BackendKind, envelope::config::EnvelopeConfig, flag::config::FlagConfig,
    folder::config::FolderConfig, message::config::MessageConfig,
};

/// Represents all existing kind of account config.
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TomlAccountConfig {
    pub default: Option<bool>,
    pub email: String,
    pub display_name: Option<String>,
    pub signature: Option<String>,
    pub signature_delim: Option<String>,
    pub downloads_dir: Option<PathBuf>,
    pub backend: Option<BackendKind>,

    #[cfg(feature = "account-sync")]
    pub sync: Option<SyncConfig>,
    #[cfg(feature = "pgp")]
    pub pgp: Option<PgpConfig>,

    pub folder: Option<FolderConfig>,
    pub envelope: Option<EnvelopeConfig>,
    pub flag: Option<FlagConfig>,
    pub message: Option<MessageConfig>,

    #[cfg(feature = "imap")]
    pub imap: Option<ImapConfig>,
    #[cfg(feature = "maildir")]
    pub maildir: Option<MaildirConfig>,
    #[cfg(feature = "notmuch")]
    pub notmuch: Option<NotmuchConfig>,
    #[cfg(feature = "smtp")]
    pub smtp: Option<SmtpConfig>,
    #[cfg(feature = "sendmail")]
    pub sendmail: Option<SendmailConfig>,
}

impl TomlAccountConfig {
    #[cfg(feature = "folder-add")]
    pub fn add_folder_kind(&self) -> Option<&BackendKind> {
        self.folder
            .as_ref()
            .and_then(|folder| folder.add.as_ref())
            .and_then(|add| add.backend.as_ref())
            .or(self.backend.as_ref())
    }

    #[cfg(feature = "folder-list")]
    pub fn list_folders_kind(&self) -> Option<&BackendKind> {
        self.folder
            .as_ref()
            .and_then(|folder| folder.list.as_ref())
            .and_then(|list| list.backend.as_ref())
            .or(self.backend.as_ref())
    }

    #[cfg(feature = "folder-expunge")]
    pub fn expunge_folder_kind(&self) -> Option<&BackendKind> {
        self.folder
            .as_ref()
            .and_then(|folder| folder.expunge.as_ref())
            .and_then(|expunge| expunge.backend.as_ref())
            .or(self.backend.as_ref())
    }

    #[cfg(feature = "folder-purge")]
    pub fn purge_folder_kind(&self) -> Option<&BackendKind> {
        self.folder
            .as_ref()
            .and_then(|folder| folder.purge.as_ref())
            .and_then(|purge| purge.backend.as_ref())
            .or(self.backend.as_ref())
    }

    #[cfg(feature = "folder-delete")]
    pub fn delete_folder_kind(&self) -> Option<&BackendKind> {
        self.folder
            .as_ref()
            .and_then(|folder| folder.delete.as_ref())
            .and_then(|delete| delete.backend.as_ref())
            .or(self.backend.as_ref())
    }

    #[cfg(feature = "envelope-get")]
    pub fn get_envelope_kind(&self) -> Option<&BackendKind> {
        self.envelope
            .as_ref()
            .and_then(|envelope| envelope.get.as_ref())
            .and_then(|get| get.backend.as_ref())
            .or(self.backend.as_ref())
    }

    #[cfg(feature = "envelope-list")]
    pub fn list_envelopes_kind(&self) -> Option<&BackendKind> {
        self.envelope
            .as_ref()
            .and_then(|envelope| envelope.list.as_ref())
            .and_then(|list| list.backend.as_ref())
            .or(self.backend.as_ref())
    }

    #[cfg(feature = "envelope-watch")]
    pub fn watch_envelopes_kind(&self) -> Option<&BackendKind> {
        self.envelope
            .as_ref()
            .and_then(|envelope| envelope.watch.as_ref())
            .and_then(|watch| watch.backend.as_ref())
            .or(self.backend.as_ref())
    }

    #[cfg(feature = "flag-add")]
    pub fn add_flags_kind(&self) -> Option<&BackendKind> {
        self.flag
            .as_ref()
            .and_then(|flag| flag.add.as_ref())
            .and_then(|add| add.backend.as_ref())
            .or(self.backend.as_ref())
    }

    #[cfg(feature = "flag-set")]
    pub fn set_flags_kind(&self) -> Option<&BackendKind> {
        self.flag
            .as_ref()
            .and_then(|flag| flag.set.as_ref())
            .and_then(|set| set.backend.as_ref())
            .or(self.backend.as_ref())
    }

    #[cfg(feature = "flag-remove")]
    pub fn remove_flags_kind(&self) -> Option<&BackendKind> {
        self.flag
            .as_ref()
            .and_then(|flag| flag.remove.as_ref())
            .and_then(|remove| remove.backend.as_ref())
            .or(self.backend.as_ref())
    }

    #[cfg(feature = "message-add")]
    pub fn add_message_kind(&self) -> Option<&BackendKind> {
        self.message
            .as_ref()
            .and_then(|msg| msg.write.as_ref())
            .and_then(|add| add.backend.as_ref())
            .or(self.backend.as_ref())
    }

    #[cfg(feature = "message-peek")]
    pub fn peek_messages_kind(&self) -> Option<&BackendKind> {
        self.message
            .as_ref()
            .and_then(|message| message.peek.as_ref())
            .and_then(|peek| peek.backend.as_ref())
            .or(self.backend.as_ref())
    }

    #[cfg(feature = "message-get")]
    pub fn get_messages_kind(&self) -> Option<&BackendKind> {
        self.message
            .as_ref()
            .and_then(|message| message.read.as_ref())
            .and_then(|get| get.backend.as_ref())
            .or(self.backend.as_ref())
    }

    #[cfg(feature = "message-copy")]
    pub fn copy_messages_kind(&self) -> Option<&BackendKind> {
        self.message
            .as_ref()
            .and_then(|message| message.copy.as_ref())
            .and_then(|copy| copy.backend.as_ref())
            .or(self.backend.as_ref())
    }

    #[cfg(feature = "message-move")]
    pub fn move_messages_kind(&self) -> Option<&BackendKind> {
        self.message
            .as_ref()
            .and_then(|message| message.move_.as_ref())
            .and_then(|move_| move_.backend.as_ref())
            .or(self.backend.as_ref())
    }

    #[cfg(feature = "message-delete")]
    pub fn delete_messages_kind(&self) -> Option<&BackendKind> {
        self.message
            .as_ref()
            .and_then(|message| message.delete.as_ref())
            .and_then(|delete| delete.backend.as_ref())
            .or(self.backend.as_ref())
    }

    #[cfg(any(feature = "message-send", feature = "template-send"))]
    pub fn send_message_kind(&self) -> Option<&BackendKind> {
        self.message
            .as_ref()
            .and_then(|msg| msg.send.as_ref())
            .and_then(|send| send.backend.as_ref())
            .or(self.backend.as_ref())
    }

    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut used_backends = HashSet::default();

        if let Some(ref kind) = self.backend {
            used_backends.insert(kind);
        }

        if let Some(ref folder) = self.folder {
            used_backends.extend(folder.get_used_backends());
        }

        #[cfg(feature = "envelope-subcmd")]
        if let Some(ref envelope) = self.envelope {
            used_backends.extend(envelope.get_used_backends());
        }

        #[cfg(feature = "flag-subcmd")]
        if let Some(ref flag) = self.flag {
            used_backends.extend(flag.get_used_backends());
        }

        #[cfg(feature = "message-subcmd")]
        if let Some(ref msg) = self.message {
            used_backends.extend(msg.get_used_backends());
        }

        used_backends
    }
}
