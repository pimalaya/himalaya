use anyhow::Result;
use pimalaya_toolbox::stream::imap::ImapSession;

use crate::{account::Account, config::ImapConfig};

pub type ImapAccount = Account<ImapConfig>;

impl ImapAccount {
    pub fn new_imap_session(&self) -> Result<ImapSession> {
        ImapSession::new(
            self.backend.url.clone(),
            self.backend.tls.clone().try_into()?,
            self.backend.starttls,
            self.backend.sasl.clone().try_into()?,
        )
    }
}
