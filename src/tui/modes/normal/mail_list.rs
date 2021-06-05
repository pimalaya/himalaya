use crate::config::tui::block_data::BlockDataConfig;
use crate::imap::model::ImapConnector;
use crate::msg::model::Msgs;
use crate::tui::modes::block_data::BlockData;
use crate::tui::modes::state_wrappers::{TableStateWrapper, TableWrapperFuncs};

use tui_rs::layout::Constraint;
use tui_rs::style::{Color, Modifier, Style};
use tui_rs::widgets::{Block, Row, Table, TableState};

use super::widgets::mail_entry::MailEntry;

// ============
// Structs
// ============
pub struct MailList {
    pub block_data: BlockData,
    mails:          Vec<MailEntry>,
    header:         Vec<String>,

    pub state: TableStateWrapper,
}

impl MailList {
    pub fn new(title: String, config: &BlockDataConfig) -> Self {
        Self {
            block_data: BlockData::new(title, config),
            mails:      Vec::new(),
            header:     vec![
                String::from("UID"),
                String::from("Flags"),
                String::from("Date"),
                String::from("Sender"),
                String::from("Subject"),
            ],
            state:      TableStateWrapper::new(),
        }
    }

    pub fn set_mails(
        &mut self,
        imap_conn: &mut ImapConnector,
        mbox: &str,
    ) -> Result<(), &str> {
        self.mails.clear();
        let msgs = match imap_conn.msgs(&mbox) {
            Ok(msgs) => msgs,
            Err(_) => return Err("Couldn't get the messages from the mailbox."),
        };

        let msgs = match msgs {
            Some(ref fetches) => Msgs::from(fetches).0,
            None => Msgs::new().0,
        };

        for message in msgs.iter() {
            let mail_entry = MailEntry::new(
                message.uid.to_string(),
                message.flags.to_string(),
                message.date.clone(),
                message.sender.clone(),
                message.subject.clone(),
            );

            self.mails.push(mail_entry);
        }

        // reset the selection
        self.state.reset();
        self.state.update_length(self.mails.len());

        Ok(())
    }

    pub fn get_current_mail(&self) -> MailEntry {
        // We can be sure that the mail exists
        self.mails[self.state.get_selected_index()].clone()
    }

    // TODO: Make sure that it displays really only the needed one, not too
    // much
    // Idea:
    // https://docs.rs/tui/0.15.0/tui/widgets/trait.StatefulWidget.html
    // pub fn widget(&mut self, height: u16) -> Table {
    pub fn widget(&self) -> Table<'static> {
        // convert the header into a row
        let header = Row::new(self.header.clone())
            .bottom_margin(1)
            .style(Style::default().add_modifier(Modifier::UNDERLINED));

        // convert all mails into Rows
        let mails: Vec<Row> = self
            .mails
            .iter()
            .map(|mail| Row::new(Vec::from(mail)))
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

impl TableWrapperFuncs for MailList {

    fn move_cursor(&mut self, offset: i32) {
        self.state.move_cursor(offset);
    }

    fn set_cursor(&mut self, index: Option<usize>) {
        self.state.set_cursor(index);
    }

    fn get_state(&mut self) -> &mut TableState {
        &mut self.state.state
    }
}
