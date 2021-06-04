use tui_rs::widgets::{Block, List, ListItem, ListState};

use crate::config::tui::block_data::BlockDataConfig;
use crate::tui::modes::block_data::BlockData;
use crate::tui::modes::list_state_wrapper::ListStateWrapper;

// ===========
// Struct
// ===========
pub struct MailContent {
    content:        Vec<String>,
    pub block_data: BlockData,
    pub state:      ListStateWrapper,
}

impl MailContent {
    pub fn new(config: &BlockDataConfig) -> Self {
        Self {
            block_data: BlockData::new(String::from("Mail Content"), config),
            content:    Vec::new(),
            state:      ListStateWrapper::new(),
        }
    }

    pub fn move_cursor(&mut self, offset: i32) {
        self.state.move_cursor(offset);
    }

    pub fn get_state(&mut self) -> &mut ListState {
        &mut self.state.state
    }

    pub fn set_content(&mut self, new_content: &str) {
        self.content.clear();

        for line in new_content.lines() {
            self.content.push(line.to_string());
        }

    }

    pub fn widget(&self) -> List<'static> {
        let block = Block::from(self.block_data.clone());
        let items: Vec<ListItem> = self
            .content
            .iter()
            .map(|line| ListItem::new(line.to_string()))
            .collect();

        List::new(items).block(block)
    }
}
