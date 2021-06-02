use crate::config::tui::BlockDataConfig;
use crate::tui::modes::block_data::BlockData;
use tui_rs::widgets::{Block, Row, Table};

// ============
// Structs
// ============
pub struct Attachments {
    block_data: BlockData,
}

impl Attachments {
    pub fn new(block_config: &BlockDataConfig) -> Self {
        Self {
            block_data: BlockData::new(
                String::from("Attachments"),
                block_config,
            ),
        }
    }

    pub fn widget(&self) -> Table<'static> {

        let block = Block::from(self.block_data.clone());

        Table::new(vec![Row::new(vec!["Yeet"])])
            .block(block)
    }
}
