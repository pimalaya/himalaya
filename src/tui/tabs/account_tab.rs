use crate::config::model::Account;
use crate::tui::model::{TuiMode, TuiError};

use super::data::normal_data::NormalData;
use super::data::viewer_data::ViewerData;
use super::data::writer_data::WriterData;

// ============
// Structs
// ============
pub struct AccountTab<'tab> {
    pub account: Account,
    pub mode:    TuiMode,

    pub normal: NormalData,
    pub viewer: ViewerData<'tab>,
    pub writer: WriterData,
}

impl<'tab> AccountTab<'tab> {
    pub fn new(account: Account, mode: TuiMode) -> Result<Self, TuiError> {

        let normal = match NormalData::new(&account) {
            Ok(normal_data) => normal_data,
            Err(err) => return Err(err),
        };

        Ok(Self {
            account,
            mode,
            normal,
            viewer: ViewerData::default(),
            writer: WriterData::default(),
        })
    }
}
