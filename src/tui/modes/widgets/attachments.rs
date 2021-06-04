use crate::config::tui::block_data::BlockDataConfig;
use crate::tui::modes::block_data::BlockData;
use crate::tui::modes::table_state_wrapper::TableStateWrapper;

use tui_rs::widgets::{Block, Row, Table, TableState};

// ============
// Structs
// ============
pub struct Attachments {
    pub block_data: BlockData,

    header: Vec<String>,

    pub state: TableStateWrapper,
}

impl Attachments {
    pub fn new(config: &BlockDataConfig) -> Self {
        Self {
            block_data: BlockData::new(String::from("Attachments"), config),
            state:      TableStateWrapper::new(),
            header:     Vec::new(),
        }
    }

    pub fn move_cursor(&mut self, offset: i32) {
        self.state.move_cursor(offset);
    }

    pub fn get_n(&mut self) -> &mut TableState {
        &mut self.state.state
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
