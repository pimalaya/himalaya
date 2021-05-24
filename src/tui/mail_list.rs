use super::mail_frame::MailFrame;
use super::block_data::BlockData;

use crate::imap::model::ImapConnector;
use crate::msg::model::Msgs;

use tui_rs::widgets::{Row, Table, Block};
use tui_rs::layout::{ Constraint };

pub struct MailList<'maillist> {
    pub block_data: BlockData,
    mails: Vec<Row<'maillist>>,
    header: Row<'maillist>,
}

impl<'maillist> MailList<'maillist> {

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

    pub fn widget(&mut self) -> Table {

        Table::new(self.mails.clone())
            .block(self.block())
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

impl<'maillist> MailFrame for MailList<'maillist> {

    fn new(title: String) -> Self {
        Self {
            block_data: BlockData::new(title),
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

    fn block(&self) -> Block {
        self.block_data.clone().into()
    }
}
