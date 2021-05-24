use super::block_data::BlockData;
use super::mail_frame::MailFrame;

use crate::imap::model::ImapConnector;
use crate::msg::model::Msgs;

use tui_rs::layout::Constraint;
use tui_rs::style::{Color, Style};
use tui_rs::widgets::{Block, Row, Table};

pub struct MailList<'maillist> {
    pub block_data: BlockData,
    mails: Vec<Row<'maillist>>,
    header: Row<'maillist>,
    select_index: usize,
}

impl<'maillist> MailList<'maillist> {
    pub fn set_mails(
        &mut self,
        imap_conn: &mut ImapConnector,
        mbox: &str,
    ) -> Result<(), &str> {
        self.mails.clear();
        let msgs = match imap_conn.msgs(&mbox) {
            Ok(msgs) => msgs,
            Err(_) => {
                return Err("Couldn't get the messages from the mailbox.")
            }
        };

        let msgs = match msgs {
            Some(ref fetches) => Msgs::from(fetches).0,
            None => Msgs::new().0,
        };

        for message in msgs.iter() {
            let row = vec![
                message.uid.to_string().clone(),
                message.flags.to_string().clone(),
                message.date.clone(),
                message.sender.clone(),
                message.subject.clone(),
            ];

            self.mails.push(Row::new(row));
        }

        self.mark_selected_index();

        Ok(())
    }

    pub fn mark_selected_index(&mut self) {
        self.mails[self.select_index] = self.mails[self.select_index]
            .clone()
            .style(Style::default().bg(Color::Cyan));
    }

    pub fn unmark_selected_index(&mut self) {
        self.mails[self.select_index] = self.mails[self.select_index]
            .clone()
            .style(Style::default());
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

    pub fn move_selection(&mut self, offset: i32) {
        self.unmark_selected_index();

        // make sure that we don't get over the index borders of the vector.
        // In other words: Prevent that the new index doesn't goes over the
        // length of the vector.
        self.select_index = if offset < 0 {
            self.select_index.saturating_sub(offset.abs() as usize)
        } else {
            self.select_index.saturating_add(offset as usize)
        };

        if self.select_index > self.mails.len() - 1{
            self.select_index = self.mails.len() - 1;
        }

        self.mark_selected_index();
    }
}

impl<'maillist> MailFrame for MailList<'maillist> {
    fn new(title: String) -> Self {
        Self {
            block_data: BlockData::new(title),
            mails: Vec::new(),
            header: Row::new(vec!["UID", "Flags", "Date", "Sender", "Subject"])
                .bottom_margin(1),
            select_index: 0,
        }
    }

    fn block(&self) -> Block {
        self.block_data.clone().into()
    }
}
