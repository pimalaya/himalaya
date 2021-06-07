use crate::config::tui::block_data::BlockDataConfig;
use crate::tui::modes::block_data::BlockData;
use crate::tui::tabs::data::normal_data::MailEntry;

use tui_rs::layout::Constraint;
use tui_rs::style::{Color, Modifier, Style};
use tui_rs::widgets::{Block, Row, Table};

// ============
// Structs
// ============
pub struct MailList {
    pub block_data: BlockData,
    header:         Vec<String>,
}

impl MailList {
    pub fn new(title: String, config: &BlockDataConfig) -> Self {
        Self {
            block_data: BlockData::new(title, config),
            header:     vec![
                String::from("UID"),
                String::from("Flags"),
                String::from("Date"),
                String::from("Sender"),
                String::from("Subject"),
            ],
        }
    }

    pub fn widget(&self, mails: &Vec<MailEntry>) -> Table<'static> {
        // convert the header into a row
        let header = Row::new(self.header.clone())
            .bottom_margin(1)
            .style(Style::default().add_modifier(Modifier::UNDERLINED));

        // convert all mails into Rows
        let mails: Vec<Row> = mails
            .iter()
            .map(|mail| Row::new(vec![
                mail.uid.to_string(),
                mail.flags.to_string(),
                mail.date.clone(),
                mail.sender.clone(),
                mail.subject.clone(),
            ]))
            .collect();

        // get the block
        let block = Block::from(self.block_data.clone());

        Table::new(mails)
            .block(block)
            .header(header)
            .widths(&[
                Constraint::Percentage(10),
                Constraint::Percentage(10),
                Constraint::Percentage(10),
                Constraint::Percentage(10),
                Constraint::Percentage(60),
            ])
            .highlight_style(Style::default().bg(Color::Blue))
    }
}
