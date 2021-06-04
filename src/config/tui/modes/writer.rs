use serde::Deserialize;

use crate::config::tui::block_data::BlockDataConfig;
use crate::tui::modes::writer::main::WriterAction;

use std::collections::HashMap;

// ============
// Structs
// ============
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct WriterConfig {
    pub header:      BlockDataConfig,
    pub attachments: BlockDataConfig,
    pub keybindings: HashMap<String, String>,

    #[serde(skip, default = "WriterConfig::default_keybindings")]
    pub default_keybindings: Vec<(&'static str, WriterAction, &'static str)>,
}

impl Default for WriterConfig {
    fn default() -> Self {
        Self {
            header: BlockDataConfig::default(),
            attachments: BlockDataConfig::default(),
            keybindings: HashMap::new(),
            default_keybindings: WriterConfig::default_keybindings(),
        }
    }
}

impl WriterConfig {
    fn default_keybindings() -> Vec<(&'static str, WriterAction, &'static str)>
    {
        vec![
            ("set_bcc", WriterAction::SetBcc, "b"),
            ("set_cc", WriterAction::SetCc, "c"),
            ("set_in_reply_to", WriterAction::SetReplyTo, "r"),
            ("set_subject", WriterAction::SetSubject, "s"),
            ("set_to", WriterAction::SetTo, "t"),
            ("quit", WriterAction::Quit, "q"),
        ]
    }
}
