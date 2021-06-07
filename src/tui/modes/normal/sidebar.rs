use crate::config::tui::block_data::BlockDataConfig;
use crate::tui::modes::block_data::BlockData;

use tui_rs::layout::Constraint;
use tui_rs::style::{Color, Style};
use tui_rs::widgets::{Block, Row, Table};

// ============
// Structs
// ============
pub struct Sidebar {
    pub block_data: BlockData,
    header:         Vec<String>,
}

impl Sidebar {
    pub fn new(title: String, config: &BlockDataConfig) -> Self {
        Self {
            header:     vec![String::from("Mailbox"), String::from("Flags")],
            block_data: BlockData::new(title, config),
        }
    }

    pub fn widget<'mbox>(&self, mboxes: &Vec<String>) -> Table<'static> {
        let rows: Vec<Row> = mboxes
            .iter()
            .map(|mailbox| Row::new(vec![mailbox.to_string()]))
            .collect();

        assert!(mboxes.len() > 0);

        let block = Block::from(self.block_data.clone());
        let header = Row::new(self.header.clone()).bottom_margin(1);

        Table::new(rows)
            .block(block)
            .header(header)
            .widths(&[Constraint::Percentage(70), Constraint::Percentage(30)])
            .highlight_style(Style::default().bg(Color::Blue))
    }
}
