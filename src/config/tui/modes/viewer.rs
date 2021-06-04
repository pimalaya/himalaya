use serde::Deserialize;

use crate::config::tui::block_data::BlockDataConfig;
use crate::tui::modes::viewer::main::ViewerAction;

use std::collections::HashMap;

// ============
// Structs
// ============
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ViewerConfig {
    pub attachments: BlockDataConfig,
    pub header:      BlockDataConfig,
    pub mailcontent: BlockDataConfig,
    pub keybindings: HashMap<String, String>,

    #[serde(skip, default = "ViewerConfig::default_keybindings")]
    pub default_keybindings: Vec<(&'static str, ViewerAction, &'static str)>,
}

impl Default for ViewerConfig {
    fn default() -> Self {
        Self {
            attachments:         BlockDataConfig::default(),
            header:              BlockDataConfig::default(),
            mailcontent:         BlockDataConfig::default(),
            keybindings:         HashMap::new(),
            default_keybindings: ViewerConfig::default_keybindings(),
        }
    }
}

impl ViewerConfig {
    fn default_keybindings() -> Vec<(&'static str, ViewerAction, &'static str)>
    {
        vec![
            ("quit", ViewerAction::Quit, "q"),
            ("quit", ViewerAction::Quit, "h"),
            ("toggle_attachment", ViewerAction::ToggleAttachment, "a"),
        ]
    }
}
