use anyhow::Result;
use pimalaya_toolbox::stream::jmap::JmapSession;

use crate::{account::Account, config::JmapConfig};

pub type JmapAccount = Account<JmapConfig>;

impl JmapAccount {
    pub fn new_jmap_session(&self) -> Result<JmapSession> {
        JmapSession::new(
            self.backend.url.clone(),
            self.backend.tls.clone().try_into()?,
            self.backend.auth.clone().try_into()?,
        )
    }
}
