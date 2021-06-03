use serde::Deserialize;

use crate::config::tui::block_data::BlockDataConfig;
use crate::tui::modes::writing_mail::main::WritingMailAction;

use std::collections::HashMap;

// ============
// Structs
// ============
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct WritingMailConfig {
    pub mail_credits: BlockDataConfig,
    pub attachments: BlockDataConfig,
    pub keybindings: HashMap<String, String>,

    #[serde(skip, default = "WritingMailConfig::default_keybindings")]
    pub default_keybindings: Vec<(&'static str, WritingMailAction, &'static str)>,
}

impl Default for WritingMailConfig {
    fn default() -> Self {
        Self {
            mail_credits: BlockDataConfig::default(),
            attachments: BlockDataConfig::default(),
            keybindings: HashMap::new(),
            default_keybindings: WritingMailConfig::default_keybindings(),
        }
    }
}

impl WritingMailConfig {

    fn default_keybindings() -> Vec<(&'static str, WritingMailAction, &'static str)> {
        vec![
            ("set_bcc",         WritingMailAction::SetBcc, "b"),
            ("set_cc",          WritingMailAction::SetBcc, "c"),
            ("set_in_reply_to", WritingMailAction::SetBcc, "r"),
            ("set_subject",     WritingMailAction::SetBcc, "s"),
            ("set_to",          WritingMailAction::SetBcc, "t"),
            ("quit",            WritingMailAction::Quit,   "q"),
        ]
    }
}
