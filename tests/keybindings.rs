use himalaya::config::tui::{TuiConfig};

#[test]
pub fn test_if_special_keybinding() {
    assert!(TuiConfig::is_special_keybinding("<C-a>"));
    assert!(TuiConfig::is_special_keybinding("<CR>"));
    assert!(TuiConfig::is_special_keybinding("<Esc>"));
    assert!(TuiConfig::is_special_keybinding("<C-Space>"));
    assert!(TuiConfig::is_special_keybinding("<C-Home>"));
    assert!(TuiConfig::is_special_keybinding("<C-S>"));
    assert!(!TuiConfig::is_special_keybinding("<C--Space>"));
    assert!(!TuiConfig::is_special_keybinding("<C-%>"));
    assert!(!TuiConfig::is_special_keybinding("<C-asdfaef>"));
}
