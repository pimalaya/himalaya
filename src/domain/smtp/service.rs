use std::convert::TryInto;

use anyhow::Result;
use lettre::{
    self,
    transport::smtp::{
        client::{Tls, TlsParameters},
        SmtpTransport,
    },
    Transport,
};
use log::debug;

use crate::{config::entity::Account, domain::msg::Msg};

pub trait SmtpServiceInterface {
    fn send(&mut self, msg: &Msg) -> Result<()>;
}

pub struct SmtpService<'a> {
    account: &'a Account,
    transport: Option<SmtpTransport>,
}

impl<'a> SmtpService<'a> {
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
    fn send(&mut self, msg: &Msg) -> Result<()> {
        debug!("sending message…");
        self.transport()?.send(&msg.try_into()?)?;
        Ok(())
    }
}

impl<'a> From<&'a Account> for SmtpService<'a> {
    fn from(account: &'a Account) -> Self {
        debug!("init SMTP service");
        Self {
            account,
            transport: None,
        }
    }
}
