use anyhow::Result;
use lettre::{
    self,
    transport::{smtp::client::Tls, smtp::client::TlsParameters, smtp::SmtpTransport},
    Transport,
};

use crate::config::model::Account;

pub trait SMTPServiceInterface<'a> {
    fn send(&self, msg: &lettre::Message) -> Result<()>;
}

pub struct SMTPService<'a> {
    account: &'a Account,
}

impl<'a> SMTPService<'a> {
    pub fn init(account: &'a Account) -> Self {
        Self { account }
    }
}

impl<'a> SMTPServiceInterface<'a> for SMTPService<'a> {
    fn send(&self, msg: &lettre::Message) -> Result<()> {
        let smtp_relay = if self.account.smtp_starttls() {
            SmtpTransport::starttls_relay
        } else {
            SmtpTransport::relay
        };

        let tls = TlsParameters::builder(self.account.smtp_host.to_string())
            .dangerous_accept_invalid_hostnames(self.account.smtp_insecure())
            .dangerous_accept_invalid_certs(self.account.smtp_insecure())
            .build()?;
        let tls = if self.account.smtp_starttls() {
            Tls::Required(tls)
        } else {
            Tls::Wrapper(tls)
        };

        smtp_relay(&self.account.smtp_host)?
            .port(self.account.smtp_port)
            .tls(tls)
            .credentials(self.account.smtp_creds()?)
            .build()
            .send(msg)?;

        Ok(())
    }
}
