use tui_rs::widgets::{Block, Row, Table, TableState};
use crate::config::tui::BlockDataConfig;
use crate::tui::modes::block_data::BlockData;

// ============
// Structs
// ============
struct Files {
    block_data: BlockData,
}

impl Files {
    pub fn new(block_config: &BlockDataConfig) -> Self {
        Self {

        }
    }
}
