use crate::imap::model::ImapConnector;
use crate::mbox::model::{Mbox, Mboxes};

use tui_rs::layout::Constraint;
use tui_rs::widgets::{Row, Table};

use super::model::MailFrame;

pub struct Sidebar {
    pub frame: MailFrame,
    mailboxes: Vec<Vec<String>>,
    header: Vec<String>,
}

impl Sidebar {
    pub fn new(frame: MailFrame) -> Self {
        Self {
            frame,
            mailboxes: Vec::new(),
            header: vec![String::from("Mailbox"), String::from("Flags")],
        }
    }

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

        for mailbox in &self.mailboxes {
            rows.push(Row::new(mailbox.clone()));
        }

        Table::new(rows)
            .block(self.frame.block())
            .header(Row::new(self.header.clone()).bottom_margin(1))
            .widths(&[Constraint::Percentage(70), Constraint::Percentage(30)])
    }
}
