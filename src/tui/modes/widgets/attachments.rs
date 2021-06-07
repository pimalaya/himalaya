use crate::config::tui::block_data::BlockDataConfig;
use crate::tui::modes::block_data::BlockData;

use tui_rs::widgets::{Block, Row, Table};

// ============
// Structs
// ============
pub struct Attachments {
    pub block_data: BlockData,
}

impl Attachments {
    pub fn new(config: &BlockDataConfig) -> Self {
        Self {
            block_data: BlockData::new(String::from("Attachments"), config),
        }
    }

    pub fn widget(&self) -> Table<'static> {
        let block = Block::from(self.block_data.clone());

        Table::new(vec![
            Row::new(vec!["Attachment 1"]),
            Row::new(vec!["Attachment 2"]),
            Row::new(vec!["Attachment 3"]),
        ])
        .block(block)
    }
}
