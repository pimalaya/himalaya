use crate::imap::model::ImapConnector;
use crate::mbox::model::Mboxes;
use crate::config::tui::BlockDataConfig;
use crate::tui::modes::block_data::BlockData;

use tui_rs::layout::Constraint;
use tui_rs::style::{Color, Style};
use tui_rs::widgets::{Block, Row, Table, TableState};

pub struct Sidebar {
    pub block_data: BlockData,
    mailboxes: Vec<Vec<String>>,
    header: Vec<String>,

    pub state: TableState,
}

impl Sidebar {
    pub fn new(title: String, config: &BlockDataConfig) -> Self {
        Self {
            mailboxes: Vec::new(),
            header: vec![String::from("Mailbox"), String::from("Flags")],
            block_data: BlockData::new(title, config),
            state: TableState::default(),
        }
    }

    pub fn mailboxes(&self) -> Vec<Vec<String>> {
        self.mailboxes.clone()
    }

    pub fn set_mailboxes(&mut self, imap_conn: &mut ImapConnector) -> Result<(), &str> {
        // ----------------
        // Preparation
        // ---------------- */
        // get the mailboxes.
        let mbox_names = match imap_conn.list_mboxes() {
            Ok(names) => names,
            Err(_) => return Err("Couldn't load the mailboxes."),
        };

        let mailboxes = Mboxes::from(&mbox_names).0;
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

        // reset the selection
        self.state = TableState::default();
        self.state.select(Some(0));

        Ok(())
    }

    pub fn move_selection(&mut self, offset:i32) {
        let new_selection = match self.state.selected() {
            Some(old_selection) => {
                let mut selection = if offset < 0 {
                    old_selection.saturating_sub(offset.abs() as usize)
                } else {
                    old_selection.saturating_add(offset as usize)
                };

                if selection > self.mailboxes.len() - 1 {
                    selection = self.mailboxes.len() - 1;
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

    pub fn widget(&self) -> Table<'static> {
        let rows: Vec<Row> = self.mailboxes
            .iter()
            .map(|mailbox| Row::new(mailbox.to_vec()))
            .collect();

        let block = Block::from(self.block_data.clone());
        let header = Row::new(self.header.clone()).bottom_margin(1);

        Table::new(rows)
            .block(block)
            .header(header)
            .widths(&[Constraint::Percentage(70), Constraint::Percentage(30)])
            .highlight_style(Style::default().bg(Color::Blue))
    }
}
