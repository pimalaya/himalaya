use super::account_tab::AccountTab;

use tui_rs::style::Style;

// ============
// Structs
// ============
pub struct TabBar<'bar> {
    pub tab_accounts: Vec<AccountTab<'bar>>,
    account_index:    usize,
    divider:          String,
    highlight_style:  Style,
}

impl<'bar> TabBar<'bar> {

    pub fn new() -> Self {
        Self {
            tab_accounts:    Vec::new(),
            divider:         String::from("|"),
            highlight_style: Style::default(),
            account_index:   0,
        }
    }

    pub fn set_divider(&mut self, divider: &str) {
        self.divider = divider.to_string();
    }

    pub fn set_highlight_style(&mut self, style: Style) {
        self.highlight_style = style;
    }

    pub fn select_account(&mut self, index: Option<usize>) {
        if let Some(index) = index {
            if index > self.tab_accounts.len() - 1 {
                self.account_index = self.tab_accounts.len() - 1;
            } else {
                self.account_index = index;
            }
        } else {
            self.account_index = self.tab_accounts.len() - 1;
        }
    }
}
