use super::model::MailFrame;

pub struct MailList {
    pub frame: MailFrame,
}

impl<'maillist> MailList {

    pub fn new(frame: MailFrame) -> Self {
        Self { frame } 
    }
}
