
// ============
// Structs
// ============
pub struct Attachment {
    pub filename: String,
    pub body_raw: Vec<u8>,
}

impl Attachment {
    pub fn new(filename: String, body_raw: Vec<u8>) -> Self {
        Self {
            filename,
            body_raw,
        }
    }
}

impl Default for Attachment {
    fn default() -> Self {
        Self {
            filename: String::new(),
            body_raw: Vec::new(),
        }
    }
}
