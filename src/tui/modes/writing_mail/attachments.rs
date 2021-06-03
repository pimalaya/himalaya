use crate::config::tui::block_data::BlockDataConfig;
use crate::tui::modes::block_data::BlockData;
use crate::tui::modes::table_state_wrapper::TableStateWrapper;

use tui_rs::widgets::{Block, Row, Table, TableState};

// ============
// Structs
// ============
pub struct Attachments {
    block_data: BlockData,

    pub state: TableStateWrapper,
}

impl Attachments {
    pub fn new(block_config: &BlockDataConfig) -> Self {
        Self {
            block_data: BlockData::new(
                String::from("Attachments"),
                block_config,
            ),
            state: TableStateWrapper::new(),
        }
    }

    pub fn widget(&self) -> Table<'static> {
        let block = Block::from(self.block_data.clone());

        Table::new(vec![Row::new(vec!["Yeet"])]).block(block)
    }

    pub fn get_state(&mut self) -> &mut TableState {
        &mut self.state.state
    }

    pub fn move_selection(&mut self, offset: i32) {
        self.state.move_selection(offset);
    }
}
