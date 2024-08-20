//! Deserialized account config module.
//!
//! This module contains the raw deserialized representation of an
//! account in the accounts section of the user configuration file.

use comfy_table::presets;
use crossterm::style::Color;
#[cfg(feature = "pgp")]
use email::account::config::pgp::PgpConfig;
#[cfg(feature = "imap")]
use email::imap::config::ImapConfig;
#[cfg(feature = "maildir")]
use email::maildir::config::MaildirConfig;
#[cfg(feature = "notmuch")]
use email::notmuch::config::NotmuchConfig;
#[cfg(feature = "sendmail")]
use email::sendmail::config::SendmailConfig;
#[cfg(feature = "smtp")]
use email::smtp::config::SmtpConfig;
use email::template::config::TemplateConfig;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::PathBuf};

use crate::{
    backend::BackendKind, envelope::config::EnvelopeConfig, flag::config::FlagConfig,
    folder::config::FolderConfig, message::config::MessageConfig, ui::map_color,
};

/// Represents all existing kind of account config.
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TomlAccountConfig {
    pub default: Option<bool>,
    pub email: String,
    pub display_name: Option<String>,
    pub signature: Option<String>,
    pub signature_delim: Option<String>,
    pub downloads_dir: Option<PathBuf>,
    pub backend: Option<BackendKind>,

    #[cfg(feature = "pgp")]
    pub pgp: Option<PgpConfig>,

    pub folder: Option<FolderConfig>,
    pub envelope: Option<EnvelopeConfig>,
    pub flag: Option<FlagConfig>,
    pub message: Option<MessageConfig>,
    pub template: Option<TemplateConfig>,

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
    pub fn folder_list_table_preset(&self) -> Option<String> {
        self.folder
            .as_ref()
            .and_then(|folder| folder.list.as_ref())
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.preset.clone())
    }

    pub fn folder_list_table_name_color(&self) -> Option<Color> {
        self.folder
            .as_ref()
            .and_then(|folder| folder.list.as_ref())
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.name_color)
    }

    pub fn folder_list_table_desc_color(&self) -> Option<Color> {
        self.folder
            .as_ref()
            .and_then(|folder| folder.list.as_ref())
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.desc_color)
    }

    pub fn envelope_list_table_preset(&self) -> Option<String> {
        self.envelope
            .as_ref()
            .and_then(|env| env.list.as_ref())
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.preset.clone())
    }

    pub fn envelope_list_table_unseen_char(&self) -> Option<char> {
        self.envelope
            .as_ref()
            .and_then(|env| env.list.as_ref())
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.unseen_char)
    }

    pub fn envelope_list_table_replied_char(&self) -> Option<char> {
        self.envelope
            .as_ref()
            .and_then(|env| env.list.as_ref())
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.replied_char)
    }

    pub fn envelope_list_table_flagged_char(&self) -> Option<char> {
        self.envelope
            .as_ref()
            .and_then(|env| env.list.as_ref())
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.flagged_char)
    }

    pub fn envelope_list_table_attachment_char(&self) -> Option<char> {
        self.envelope
            .as_ref()
            .and_then(|env| env.list.as_ref())
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.attachment_char)
    }

    pub fn envelope_list_table_id_color(&self) -> Option<Color> {
        self.envelope
            .as_ref()
            .and_then(|env| env.list.as_ref())
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.id_color)
    }

    pub fn envelope_list_table_flags_color(&self) -> Option<Color> {
        self.envelope
            .as_ref()
            .and_then(|env| env.list.as_ref())
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.flags_color)
    }

    pub fn envelope_list_table_subject_color(&self) -> Option<Color> {
        self.envelope
            .as_ref()
            .and_then(|env| env.list.as_ref())
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.subject_color)
    }

    pub fn envelope_list_table_sender_color(&self) -> Option<Color> {
        self.envelope
            .as_ref()
            .and_then(|env| env.list.as_ref())
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.sender_color)
    }

    pub fn envelope_list_table_date_color(&self) -> Option<Color> {
        self.envelope
            .as_ref()
            .and_then(|env| env.list.as_ref())
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.date_color)
    }

    pub fn add_folder_kind(&self) -> Option<&BackendKind> {
        self.folder
            .as_ref()
            .and_then(|folder| folder.add.as_ref())
            .and_then(|add| add.backend.as_ref())
            .or(self.backend.as_ref())
    }

    pub fn list_folders_kind(&self) -> Option<&BackendKind> {
        self.folder
            .as_ref()
            .and_then(|folder| folder.list.as_ref())
            .and_then(|list| list.backend.as_ref())
            .or(self.backend.as_ref())
    }

    pub fn expunge_folder_kind(&self) -> Option<&BackendKind> {
        self.folder
            .as_ref()
            .and_then(|folder| folder.expunge.as_ref())
            .and_then(|expunge| expunge.backend.as_ref())
            .or(self.backend.as_ref())
    }

    pub fn purge_folder_kind(&self) -> Option<&BackendKind> {
        self.folder
            .as_ref()
            .and_then(|folder| folder.purge.as_ref())
            .and_then(|purge| purge.backend.as_ref())
            .or(self.backend.as_ref())
    }

    pub fn delete_folder_kind(&self) -> Option<&BackendKind> {
        self.folder
            .as_ref()
            .and_then(|folder| folder.delete.as_ref())
            .and_then(|delete| delete.backend.as_ref())
            .or(self.backend.as_ref())
    }

    pub fn get_envelope_kind(&self) -> Option<&BackendKind> {
        self.envelope
            .as_ref()
            .and_then(|envelope| envelope.get.as_ref())
            .and_then(|get| get.backend.as_ref())
            .or(self.backend.as_ref())
    }

    pub fn list_envelopes_kind(&self) -> Option<&BackendKind> {
        self.envelope
            .as_ref()
            .and_then(|envelope| envelope.list.as_ref())
            .and_then(|list| list.backend.as_ref())
            .or(self.backend.as_ref())
    }

    pub fn thread_envelopes_kind(&self) -> Option<&BackendKind> {
        self.envelope
            .as_ref()
            .and_then(|envelope| envelope.thread.as_ref())
            .and_then(|thread| thread.backend.as_ref())
            .or(self.backend.as_ref())
    }

    pub fn add_flags_kind(&self) -> Option<&BackendKind> {
        self.flag
            .as_ref()
            .and_then(|flag| flag.add.as_ref())
            .and_then(|add| add.backend.as_ref())
            .or(self.backend.as_ref())
    }

    pub fn set_flags_kind(&self) -> Option<&BackendKind> {
        self.flag
            .as_ref()
            .and_then(|flag| flag.set.as_ref())
            .and_then(|set| set.backend.as_ref())
            .or(self.backend.as_ref())
    }

    pub fn remove_flags_kind(&self) -> Option<&BackendKind> {
        self.flag
            .as_ref()
            .and_then(|flag| flag.remove.as_ref())
            .and_then(|remove| remove.backend.as_ref())
            .or(self.backend.as_ref())
    }

    pub fn add_message_kind(&self) -> Option<&BackendKind> {
        self.message
            .as_ref()
            .and_then(|msg| msg.write.as_ref())
            .and_then(|add| add.backend.as_ref())
            .or(self.backend.as_ref())
    }

    pub fn peek_messages_kind(&self) -> Option<&BackendKind> {
        self.message
            .as_ref()
            .and_then(|message| message.peek.as_ref())
            .and_then(|peek| peek.backend.as_ref())
            .or(self.backend.as_ref())
    }

    pub fn get_messages_kind(&self) -> Option<&BackendKind> {
        self.message
            .as_ref()
            .and_then(|message| message.read.as_ref())
            .and_then(|get| get.backend.as_ref())
            .or(self.backend.as_ref())
    }

    pub fn copy_messages_kind(&self) -> Option<&BackendKind> {
        self.message
            .as_ref()
            .and_then(|message| message.copy.as_ref())
            .and_then(|copy| copy.backend.as_ref())
            .or(self.backend.as_ref())
    }

    pub fn move_messages_kind(&self) -> Option<&BackendKind> {
        self.message
            .as_ref()
            .and_then(|message| message.r#move.as_ref())
            .and_then(|move_| move_.backend.as_ref())
            .or(self.backend.as_ref())
    }

    pub fn delete_messages_kind(&self) -> Option<&BackendKind> {
        self.message
            .as_ref()
            .and_then(|message| message.delete.as_ref())
            .and_then(|delete| delete.backend.as_ref())
            .or(self.backend.as_ref())
    }

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

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ListAccountsTableConfig {
    pub preset: Option<String>,
    pub name_color: Option<Color>,
    pub backends_color: Option<Color>,
    pub default_color: Option<Color>,
}

impl ListAccountsTableConfig {
    pub fn preset(&self) -> &str {
        self.preset.as_deref().unwrap_or(presets::ASCII_MARKDOWN)
    }

    pub fn name_color(&self) -> comfy_table::Color {
        map_color(self.name_color.unwrap_or(Color::Green))
    }

    pub fn backends_color(&self) -> comfy_table::Color {
        map_color(self.backends_color.unwrap_or(Color::Blue))
    }

    pub fn default_color(&self) -> comfy_table::Color {
        map_color(self.default_color.unwrap_or(Color::Reset))
    }
}
