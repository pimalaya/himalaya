use super::model::MailFrame;

use crate::imap::model::ImapConnector;
use crate::msg::model::Msgs;

use tui_rs::widgets::{Row, Table};
use tui_rs::layout::Constraint;

pub struct MailList<'maillist> {
    pub frame: MailFrame,
    mails: Vec<Row<'maillist>>,
    header: Row<'maillist>,
}

impl<'maillist> MailList<'maillist> {

    pub fn new(frame: MailFrame) -> Self {
        Self {
            frame,
            mails: Vec::new(),
            header: Row::new(
                vec![
                "UID",
                "Flags",
                "Date",
                "Sender",
                "Subject",
                ])
                .bottom_margin(1)
        }
    }

    pub fn set_mails(&mut self, imap_conn: &mut ImapConnector, mbox: &str) -> Result<(), &str> {

        self.mails.clear();
        let msgs = match imap_conn.msgs(&mbox) {
            Ok(msgs) => msgs,
            Err(_) => return Err("Couldn't get the messages from the mailbox."),
        };

        let msgs = match msgs {
            Some(ref fetches) => Msgs::from(fetches).0,
            None => Msgs::new().0,
        };

        for message in &msgs {

            let row = vec![
                message.uid.to_string().clone(),
                message.flags.to_string().clone(),
                message.date.clone(),
                message.sender.clone(),
                message.subject.clone(),
            ];

            self.mails.push(Row::new(row));
        }

        Ok(())
    }

    pub fn widget(&self) -> Table {
        Table::new(self.mails.clone())
            .block(self.frame.block())
            .header(self.header.clone())
            .widths(&[
                Constraint::Percentage(10),
                Constraint::Percentage(10),
                Constraint::Percentage(10),
                Constraint::Percentage(10),
                Constraint::Percentage(60),
            ])
    }
}
