use anyhow::Result;
use lettre::{
    self,
    transport::smtp::{
        client::{Tls, TlsParameters},
        SmtpTransport,
    },
    Transport,
};

use crate::domain::account::entity::Account;

pub trait SmtpServiceInterface {
    fn send(&mut self, msg: &lettre::Message) -> Result<()>;
}

pub struct SmtpService<'a> {
    account: &'a Account,
    transport: Option<SmtpTransport>,
}

impl<'a> SmtpService<'a> {
    pub fn new(account: &'a Account) -> Result<Self> {
        Ok(Self {
            account,
            transport: None,
        })
    }

    fn transport(&mut self) -> Result<&SmtpTransport> {
        if let Some(ref transport) = self.transport {
            Ok(transport)
        } else {
            let builder = if self.account.smtp_starttls {
                SmtpTransport::starttls_relay(&self.account.smtp_host)
            } else {
                SmtpTransport::relay(&self.account.smtp_host)
            }?;

            let tls = TlsParameters::builder(self.account.smtp_host.to_owned())
                .dangerous_accept_invalid_hostnames(self.account.smtp_insecure)
                .dangerous_accept_invalid_certs(self.account.smtp_insecure)
                .build()?;
            let tls = if self.account.smtp_starttls {
                Tls::Required(tls)
            } else {
                Tls::Wrapper(tls)
            };

            self.transport = Some(
                builder
                    .tls(tls)
                    .port(self.account.smtp_port)
                    .credentials(self.account.smtp_creds()?)
                    .build(),
            );

            Ok(self.transport.as_ref().unwrap())
        }
    }
}

impl<'a> SmtpServiceInterface for SmtpService<'a> {
    fn send(&mut self, msg: &lettre::Message) -> Result<()> {
        self.transport()?.send(msg)?;
        Ok(())
    }
}
