use super::block_data::BlockData;

use crate::imap::model::ImapConnector;
use crate::msg::model::Msgs;
use crate::config::tui::BlockDataConfig;

use tui_rs::layout::Constraint;
use tui_rs::style::{Color, Style, Modifier};
use tui_rs::widgets::{Block, Row, Table, TableState};

pub struct MailList {
    pub block_data: BlockData,
    mails: Vec<Vec<String>>,
    header: Vec<String>,

    /// This variable/state can be modified by the UI which stores the current
    /// selected item + offset of the previous call. For more information, take
    /// a look at [its docs](trait.StatefulWidget.html#associatedtype.State).
    pub state: TableState,
}

impl MailList {

    pub fn new(title: String, config: &BlockDataConfig) -> Self {
        Self {
            block_data: BlockData::new(title, config),
            mails: Vec::new(),
            header: vec![
                String::from("UID"),
                String::from("Flags"),
                String::from("Date"),
                String::from("Sender"),
                String::from("Subject"),
            ],
            state: TableState::default(),
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
                message.uid.to_string(),
                message.flags.to_string(),
                message.date.clone(),
                message.sender.clone(),
                message.subject.clone(),
            ];

            self.mails.push(row);
        }

        // reset the selection
        self.state = TableState::default();
        self.state.select(Some(0));

        Ok(())
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
            .style(
                Style::default()
                    .add_modifier(Modifier::UNDERLINED)
            );

        // convert all mails into Rows
        let mails: Vec<Row> = self.mails.iter().map(|mail| Row::new(mail.to_vec())).collect();

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

    /// Move the select-row according to the offset.
    /// Positive Offset => Go down
    /// Negative Offset => Go up
    pub fn move_selection(&mut self, offset: i32) {
        let new_selection = match self.state.selected() {
            Some(old_selection) => {
                let mut selection = if offset < 0 {
                    old_selection.saturating_sub(offset.abs() as usize)
                } else {
                    old_selection.saturating_add(offset as usize)
                };

                if selection > self.mails.len() - 1 {
                    selection = self.mails.len() - 1;
                }

                selection
            }
            // If something goes wrong: Move the cursor to the middle
            None => 0,
        };

        self.state.select(Some(new_selection));
    }

    pub fn unselect(&mut self) {
        self.state.select(None);
    }
}
