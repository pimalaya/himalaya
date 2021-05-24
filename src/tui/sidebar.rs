use crate::imap::model::ImapConnector;
use crate::mbox::model::Mboxes;

use tui_rs::layout::Constraint;
use tui_rs::style::{Color, Style};
use tui_rs::widgets::{Block, Row, Table};

use super::block_data::BlockData;
use super::mail_frame::MailFrame;

pub struct Sidebar {
    mailboxes: Vec<Vec<String>>,
    header: Vec<String>,
    pub select_index: usize,
    pub block_data: BlockData,
}

impl Sidebar {
    pub fn mailboxes(&self) -> Vec<Vec<String>> {
        self.mailboxes.clone()
    }

    pub fn set_mailboxes(&mut self, imap_conn: &mut ImapConnector) -> Result<(), &str> {
        // Preparation
        let names = match imap_conn.list_mboxes() {
            Ok(names) => names,
            Err(_) => return Err("Couldn't load the mailboxes."),
        };

        let mailboxes = Mboxes::from(&names).0;

        self.mailboxes.clear();

        // Filling
        for mailbox in &mailboxes {
            //let attributes = String::new();
            // let mailbox_attributes = mailbox.attributes

            let attributes = mailbox.attributes.to_string();

            // match mailbox.attributes {
            //     NameAttribute::Marked => attributes.push('M'),
            //     NameAttribute::Unmarked => attributes.push('U'),
            //     NameAttribute::NoInferiors => attributes.push('N'),
            //     NameAttribute::NoSelect => attributes.push('S'),
            // };

            let row = vec![mailbox.name.clone(), attributes.clone()];
            self.mailboxes.push(row);
        }

        Ok(())
    }

    pub fn widget(&self) -> Table {
        let mut rows = Vec::new();

        for (index, mailbox) in self.mailboxes.iter().enumerate() {
            if index == self.select_index {
                rows.push(Row::new(mailbox.clone()).style(Style::default().bg(Color::Cyan)));
            } else {
                rows.push(Row::new(mailbox.clone()));
            }
        }

        Table::new(rows)
            .block(self.block())
            .header(Row::new(self.header.clone()).bottom_margin(1))
            .widths(&[Constraint::Percentage(70), Constraint::Percentage(30)])
    }
}

impl MailFrame for Sidebar {
    fn new(title: String) -> Self {
        Self {
            mailboxes: Vec::new(),
            header: vec![String::from("Mailbox"), String::from("Flags")],
            select_index: 0,
            block_data: BlockData::new(title),
        }
    }

    fn block(&self) -> Block {
        self.block_data.clone().into()
    }
}
