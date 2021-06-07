use crate::msg::model::Attachment;
use crate::tui::modes::state_wrappers::TableStateWrapper;

use super::shared_widgets::header::Header;

// ============
// Structs
// ============
pub struct WriterData {
    header: Header,

    attachments: Vec<Attachment>,
    attachments_state: TableStateWrapper,
}

impl Default for WriterData {
    fn default() -> Self {
        Self {
            header: Header::default(),
            attachments: Vec::new(),
            attachments_state: TableStateWrapper::new(None),
        }
    }
}
