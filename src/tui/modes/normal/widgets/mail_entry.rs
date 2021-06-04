// ============
// Structs
// ============
#[derive(Debug, Clone)]
pub struct MailEntry {
    uid:     String,
    flags:   String,
    date:    String,
    sender:  String,
    subject: String,
}

impl MailEntry {
    pub fn new(
        uid: String,
        flags: String,
        date: String,
        sender: String,
        subject: String,
    ) -> Self {
        Self {
            uid,
            flags,
            date,
            sender,
            subject,
        }
    }

    pub fn get_uid(&self) -> String {
        self.uid.clone()
    }

    pub fn get_flags(&self) -> String {
        self.flags.clone()
    }

    pub fn get_date(&self) -> String {
        self.date.clone()
    }

    pub fn get_sender(&self) -> String {
        self.sender.clone()
    }

    pub fn get_subject(&self) -> String {
        self.subject.clone()
    }

}

impl From<&MailEntry> for Vec<String> {
    fn from(mail_entry: &MailEntry) -> Vec<String> {
        vec![
            mail_entry.get_uid(),
            mail_entry.get_flags(),
            mail_entry.get_date(),
            mail_entry.get_sender(),
            mail_entry.get_subject(),
        ]
    }
}
