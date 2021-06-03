use tui_rs::widgets::{Row, Table, Block};
use tui_rs::layout::Constraint;

use crate::config::tui::BlockDataConfig;
use crate::tui::modes::block_data::BlockData;

// ============
// Structs
// ============
pub struct MailCredits {
    from: String,
    to: String,
    subject: String,
    cc: String,
    bcc: String,
    gpg: String,

    block_data: BlockData,
}

impl MailCredits {
    pub fn new(sender_mail_address: String, config: &BlockDataConfig) -> Self {
        Self {
            from: sender_mail_address,
            to: String::new(),
            subject: String::new(),
            cc: String::new(),
            bcc: String::new(),
            gpg: String::new(),

            block_data: BlockData::new(String::from("Mail Credits"), config),
        }
    }

    pub fn set_from(&mut self, from: String) {
        self.from = from;
    }

    pub fn set_to(&mut self, to: String) {
        self.to = to;
    }

    pub fn set_subject(&mut self, subject: String) {
        self.subject = subject;
    }

    pub fn set_cc(&mut self, cc: String) {
        self.cc = cc;
    }

    pub fn set_bcc(&mut self, bcc: String) {
        self.bcc = bcc;
    }

    pub fn set_gpg(&mut self, gpg: String) {
        self.gpg = gpg;
    }

    pub fn widget(&self) -> Table<'static> {
        let rows = vec![
            Row::new(vec![String::from("From:"), self.from.clone()]),
            Row::new(vec![String::from("To:"), self.to.clone()]),
            Row::new(vec![String::from("Subject:"), self.subject.clone()]),
            Row::new(vec![String::from("CC:"), self.cc.clone()]),
            Row::new(vec![String::from("BCC:"), self.bcc.clone()]),
            Row::new(vec![String::from("GPG:"), String::from("Not Implemented yet...")])
        ];

        let block = Block::from(self.block_data.clone());

        Table::new(rows)
            .block(block)
            .widths(&[Constraint::Percentage(10), Constraint::Percentage(90)])
    }
}
