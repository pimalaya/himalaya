use anyhow::Result;
use pimalaya_toolbox::stream::smtp::SmtpSession;

use crate::{account::Account, config::SmtpConfig};

pub type SmtpAccount = Account<SmtpConfig>;

impl SmtpAccount {
    pub fn new_smtp_session(&self) -> Result<SmtpSession> {
        SmtpSession::new(
            self.backend.url.clone(),
            self.backend.tls.clone().try_into()?,
            self.backend.starttls,
            self.backend.sasl.clone().try_into()?,
        )
    }
}
