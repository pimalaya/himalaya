use crate::config::tui::block_data::BlockDataConfig;
use crate::tui::modes::block_data::BlockData;

use tui_rs::widgets::{Block, Row, Table};

// ===========
// Struct
// ===========
pub struct Header {
    pub block_data: BlockData,

    bcc:      String,
    cc:       String,
    from:     String,
    // gpg:      String,
    reply_to: String,
    subject:  String,
    to:       String,
}

impl Header {
    pub fn new(config: &BlockDataConfig) -> Self {
        Self {
            block_data: BlockData::new(String::from("Header"), config),
            bcc:        String::new(),
            cc:         String::new(),
            from:       String::new(),
            reply_to:   String::new(),
            subject:    String::new(),
            to:         String::new(),
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
        .block(block)
    }
}
