
// ============
// Structs
// ============
/// This struct stores the information from an attachment:
///     1. It's filename
///     2. It's content
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
