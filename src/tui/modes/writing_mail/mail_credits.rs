use tui_rs::widgets::Table;

use crate::tui::modes::block_data::BlockData;

// ============
// Structs
// ============
struct MailCredits {
    to: String,
    subject: String,
    cc: String,
    bcc: String,
    gpg: String,
    block_data: BlockData,
}

impl MailCredits {
    pub fn new() -> Self {
        Self {
            to: String::new(),
            subject: String::new(),
            cc: String::new(),
            bcc: String::new(),
            gpg: String::new(),
            block_data: BlockData::new(),
        }
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
}
