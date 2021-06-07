// ============
// Structs
// ============
pub struct Header {
    bcc:      String,
    cc:       String,
    from:     String,
    // gpg:      String,
    reply_to: String,
    subject:  String,
    to:       String,
}

impl Header {
    pub fn new() -> Self {
        Self {
            bcc:      String::new(),
            cc:       String::new(),
            from:     String::new(),
            reply_to: String::new(),
            subject:  String::new(),
            to:       String::new(),
        }
    }
}

impl Default for Header {
    fn default() -> Self {
        Self {
            bcc:      String::new(),
            cc:       String::new(),
            from:     String::new(),
            reply_to: String::new(),
            subject:  String::new(),
            to:       String::new(),
        }
    }
}
