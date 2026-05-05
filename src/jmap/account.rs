use anyhow::Result;
use io_jmap::client::JmapClient;

use crate::{account::Account, config::JmapConfig, jmap::session::JmapSession};

pub type JmapAccount = Account<JmapConfig>;

impl JmapAccount {
    /// Establishes the JMAP session (TLS, `/.well-known/jmap` discovery)
    /// then hands the resulting stream, bearer token and discovered
    /// session off to a fresh [`JmapClient`].
    pub fn new_jmap_client(&self) -> Result<JmapClient> {
        let session = JmapSession::new(
            self.backend.server.clone(),
            self.backend.tls.clone().try_into()?,
            self.backend.auth.clone().try_into()?,
        )?;
        Ok(JmapClient::from_parts(
            session.stream,
            session.http_auth,
            session.session,
        ))
    }
}
