use super::mail_credits::MailCredits;

use crate::msg::tpl::Tpl;

// ==========
// Enums
// ==========


// ============
// Structs
// ============
pub struct WritingMail {
    template: Tpl,
}

impl WritingMail {
    pub fn new() -> Self {
        Self {
            template: Tpl::new(),
        }
    }
}
