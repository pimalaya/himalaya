// use tui_rs::widgets::{Block, List, ListItem, ListState};
use tui_rs::layout::Alignment;
use tui_rs::style::Style;
use tui_rs::text::{Span, Spans};
use tui_rs::widgets::{Block, ListState, Paragraph, Wrap};

use crate::config::tui::block_data::BlockDataConfig;
use crate::tui::modes::block_data::BlockData;
use crate::tui::modes::list_state_wrapper::ListStateWrapper;

// ===========
// Struct
// ===========
pub struct MailContent {
    content:        Vec<String>,
    pub block_data: BlockData,
    pub x_offset:   u16,
    pub y_offset:   u16,
}

impl MailContent {
    pub fn new(config: &BlockDataConfig) -> Self {
        Self {
            block_data: BlockData::new(String::from("Mail Content"), config),
            content:    Vec::new(),
            x_offset:   0,
            y_offset:   0,
        }
    }

    pub fn set_content(&mut self, new_content: &str) {
        self.content.clear();

        for line in new_content.lines() {
            self.content.push(line.to_string());
        }
    }

    pub fn add_offset(&mut self, x: u16, y: u16) {
        self.x_offset = self.x_offset.saturating_add(x);
        self.y_offset = self.y_offset.saturating_add(y);
    }

    pub fn sub_offset(&mut self, x: u16, y: u16) {
        self.x_offset = self.x_offset.saturating_sub(x);
        self.y_offset = self.y_offset.saturating_sub(y);
    }

    pub fn widget(&self) -> Paragraph<'static> {
        let block = Block::from(self.block_data.clone());

        let text: Vec<Spans> = self
            .content
            .clone()
            .iter()
            .map(|line| Spans::from(Span::raw(line.clone())))
            .collect();

        Paragraph::new(text)
            .block(block)
            .alignment(Alignment::Left)
            .style(Style::default())
            .wrap(Wrap { trim: true })
            .scroll((self.x_offset, self.y_offset))
    }
}
