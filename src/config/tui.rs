use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TuiConfig {
    pub sidebar: BlockDataConfig,
    pub mail_list: BlockDataConfig,
    pub keybindings: Option<KeybindingsConfig>,
}

// #[derive(Debug, Deserialize)]
// struct SidebarConfig {
//     border_type: Option<String>,
//     borders: Option<String>,
//     border_color: Option<String>,
// }

// #[derive(Debug, Deserialize)]
// struct MailListConfig {
//     border_type: Option<String>,
//     borders: Option<String>,
//     border_color: Option<String>,
// }

#[derive(Debug, Deserialize)]
pub struct BlockDataConfig {
    pub border_type: Option<String>,
    pub borders: Option<String>,
    pub border_color: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct KeybindingsConfig {
    pub quit: Option<String>,
}
