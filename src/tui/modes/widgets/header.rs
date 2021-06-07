use crate::config::tui::block_data::BlockDataConfig;
use crate::tui::modes::block_data::BlockData;

use tui_rs::widgets::{Block, Row, Table};
use tui_rs::layout::Constraint;

// ===========
// Struct
// ===========
pub struct Header {
    pub block_data: BlockData,
}

impl Header {
    pub fn new(config: &BlockDataConfig) -> Self {
        Self {
            block_data: BlockData::new(String::from("Header"), config),
        }
    }

    pub fn widget(&self) -> Table {
        let block = Block::from(self.block_data.clone());

        Table::new(vec![
            Row::new(vec!["To: "]),
            Row::new(vec!["From: "]),
            Row::new(vec!["Subject: "]),
            Row::new(vec!["BCC: "]),
            Row::new(vec!["CC: "]),
            Row::new(vec!["Reply To:"]),
        ])
        .widths(&[
            Constraint::Percentage(20),
            Constraint::Percentage(80),
        ])
        .block(block)
    }
}
