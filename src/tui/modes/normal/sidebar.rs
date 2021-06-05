use crate::imap::model::ImapConnector;
use crate::mbox::model::Mboxes;
use crate::config::tui::block_data::BlockDataConfig;
use crate::tui::modes::block_data::BlockData;
use crate::tui::modes::state_wrappers::{TableStateWrapper, TableWrapperFuncs};

use tui_rs::layout::Constraint;
use tui_rs::style::{Color, Style};
use tui_rs::widgets::{Block, Row, Table, TableState};

// ============
// Structs
// ============
pub struct Sidebar {
    pub block_data: BlockData,
    mailboxes: Vec<Vec<String>>,
    header: Vec<String>,

    pub state: TableStateWrapper,
}

impl Sidebar {
    pub fn new(title: String, config: &BlockDataConfig) -> Self {
        Self {
            mailboxes: Vec::new(),
            header: vec![String::from("Mailbox"), String::from("Flags")],
            block_data: BlockData::new(title, config),
            state: TableStateWrapper::new(),
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
        self.state.reset();
        self.state.update_length(self.mailboxes.len());

        Ok(())
    }

    pub fn get_current_mailbox(&self) -> String {
        self.mailboxes[self.state.get_selected_index()][0].clone()
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

impl TableWrapperFuncs for Sidebar {
    fn move_cursor(&mut self, offset:i32) {
        self.state.move_cursor(offset);
    }

    fn get_state(&mut self) -> &mut TableState {
        &mut self.state.state
    }
    
    fn set_cursor(&mut self, index: Option<usize>) {
        self.state.set_cursor(index);
    }
}
