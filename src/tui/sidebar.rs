use crate::mbox::model::{Mbox, Mboxes};
use crate::imap::model::ImapConnector;

use tui_rs::layout::Constraint;
use tui_rs::widgets::{Row, Table};

use super::model::MailFrame;

pub struct Sidebar<'sidebar> {
    pub frame: MailFrame,
    mailboxes: Vec<Row<'sidebar>>,
    header: tui_rs::widgets::Row<'sidebar>,
}

impl<'sidebar> Sidebar<'sidebar> {
    pub fn new(frame: MailFrame) -> Self {
        Self {
            frame,
            mailboxes: Vec::new(),
            header: Row::new(vec!["Mailbox", "Flags"]).bottom_margin(1),
        }
    }

    pub fn set_mailboxes(&mut self, imap_conn: &mut ImapConnector) -> Result<(), &str> {

        let names = match imap_conn.list_mboxes() {
            Ok(names) => names,
            Err(_) => return Err("Couldn't load the mailboxes."),
        };

        let mailboxes = Mboxes::from(&names).0;

        self.mailboxes.clear();

        let attributes = String::from("None");

        for mailbox in &mailboxes {
            let row = vec![
                mailbox.name.clone(),
                attributes.clone(),
            ];
            self.mailboxes.push(Row::new(row));
        }

        Ok(())
    }

    pub fn widget(&self) -> Table {
        Table::new(self.mailboxes.clone())
            .block(self.frame.block())
            .header(self.header.clone())
            .widths(&[
                Constraint::Percentage(80),
                Constraint::Percentage(20),
            ])
    }
}
