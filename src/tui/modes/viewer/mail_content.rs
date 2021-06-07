use tui_rs::layout::Alignment;
use tui_rs::widgets::{Block, Paragraph, Wrap};

use crate::config::tui::block_data::BlockDataConfig;
use crate::tui::modes::block_data::BlockData;
use crate::tui::tabs::data::viewer_data::MailContent;

// ===========
// Struct
// ===========

pub struct ContentWidget {
    pub block_data: BlockData,
}

impl ContentWidget {
    pub fn new(config: &BlockDataConfig) -> Self {

        Self {
            block_data: BlockData::new(String::from("Mail Content"), config),
        }
    }

    pub fn widget<'content>(&self, content: &MailContent<'content>) -> Paragraph<'content> {
        let block = Block::from(self.block_data.clone());
        // -----------------------
        // Highlight document
        // -----------------------
        // for line in self.content.lines() {
        //     content.push(Spans::from(Span::raw(line.to_string())));
        // }
        Paragraph::new(content.content.clone())
            .block(block)
            .alignment(Alignment::Left)
            .style(tui_rs::style::Style::default())
            .wrap(Wrap { trim: true })
            .scroll((content.x_offset.clone(), content.y_offset.clone()))
    }
}
