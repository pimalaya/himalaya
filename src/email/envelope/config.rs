use comfy_table::presets;
use crossterm::style::Color;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::{backend::BackendKind, ui::map_color};

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct EnvelopeConfig {
    pub list: Option<ListEnvelopesConfig>,
    pub thread: Option<ThreadEnvelopesConfig>,
    pub get: Option<GetEnvelopeConfig>,
}

impl EnvelopeConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(list) = &self.list {
            kinds.extend(list.get_used_backends());
        }

        if let Some(get) = &self.get {
            kinds.extend(get.get_used_backends());
        }

        kinds
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ListEnvelopesConfig {
    pub backend: Option<BackendKind>,
    pub table: Option<ListEnvelopesTableConfig>,

    #[serde(flatten)]
    pub remote: email::envelope::list::config::EnvelopeListConfig,
}

impl ListEnvelopesConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ListEnvelopesTableConfig {
    pub preset: Option<String>,

    pub unseen_char: Option<char>,
    pub replied_char: Option<char>,
    pub flagged_char: Option<char>,
    pub attachment_char: Option<char>,

    pub id_color: Option<Color>,
    pub flags_color: Option<Color>,
    pub subject_color: Option<Color>,
    pub sender_color: Option<Color>,
    pub date_color: Option<Color>,
}

impl ListEnvelopesTableConfig {
    pub fn preset(&self) -> &str {
        self.preset.as_deref().unwrap_or(presets::ASCII_MARKDOWN)
    }

    pub fn replied_char(&self, replied: bool) -> char {
        if replied {
            self.replied_char.unwrap_or('R')
        } else {
            ' '
        }
    }

    pub fn flagged_char(&self, flagged: bool) -> char {
        if flagged {
            self.flagged_char.unwrap_or('!')
        } else {
            ' '
        }
    }

    pub fn attachment_char(&self, attachment: bool) -> char {
        if attachment {
            self.attachment_char.unwrap_or('@')
        } else {
            ' '
        }
    }

    pub fn unseen_char(&self, unseen: bool) -> char {
        if unseen {
            self.unseen_char.unwrap_or('*')
        } else {
            ' '
        }
    }

    pub fn id_color(&self) -> comfy_table::Color {
        map_color(self.id_color.unwrap_or(Color::Red))
    }

    pub fn flags_color(&self) -> comfy_table::Color {
        map_color(self.flags_color.unwrap_or(Color::Reset))
    }

    pub fn subject_color(&self) -> comfy_table::Color {
        map_color(self.subject_color.unwrap_or(Color::Green))
    }

    pub fn sender_color(&self) -> comfy_table::Color {
        map_color(self.sender_color.unwrap_or(Color::Blue))
    }

    pub fn date_color(&self) -> comfy_table::Color {
        map_color(self.date_color.unwrap_or(Color::DarkYellow))
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct ThreadEnvelopesConfig {
    pub backend: Option<BackendKind>,

    #[serde(flatten)]
    pub remote: email::envelope::thread::config::EnvelopeThreadConfig,
}

impl ThreadEnvelopesConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct GetEnvelopeConfig {
    pub backend: Option<BackendKind>,
}

impl GetEnvelopeConfig {
    pub fn get_used_backends(&self) -> HashSet<&BackendKind> {
        let mut kinds = HashSet::default();

        if let Some(kind) = &self.backend {
            kinds.insert(kind);
        }

        kinds
    }
}
