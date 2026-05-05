use anyhow::Result;
use io_imap::client::ImapClient;

use crate::{account::Account, config::ImapConfig, imap::session::ImapSession};

pub type ImapAccount = Account<ImapConfig>;

impl ImapAccount {
    /// Opens the IMAP connection (TCP/TLS/STARTTLS, greeting, SASL),
    /// then hands the established stream and context off to a fresh
    /// [`ImapClient`].
    pub fn new_imap_client(&self) -> Result<ImapClient> {
        let session = ImapSession::new(
            self.backend.url.clone(),
            self.backend.tls.clone().try_into()?,
            self.backend.starttls,
            self.backend.sasl.clone().try_into()?,
        )?;
        Ok(ImapClient::from_parts(session.stream, session.context))
    }
}
