use anyhow::Result;
use io_smtp::client::SmtpClient;
use pimalaya_stream::std::smtp::SmtpSession;

use crate::{account::Account, config::SmtpConfig};

pub type SmtpAccount = Account<SmtpConfig>;

impl SmtpAccount {
    /// Opens the SMTP connection (TCP/TLS/STARTTLS, greeting, EHLO,
    /// SASL), then hands the established stream off to a fresh
    /// [`SmtpClient`]. SMTP send is stateless after auth, so no
    /// session context needs to follow the stream.
    pub fn new_smtp_client(&self) -> Result<SmtpClient> {
        let session = SmtpSession::new(
            self.backend.url.clone(),
            self.backend.tls.clone().try_into()?,
            self.backend.starttls,
            self.backend.sasl.clone().try_into()?,
        )?;
        Ok(SmtpClient::new(session.stream))
    }
}
