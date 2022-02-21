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

use crate::{config::AccountConfig, msg::Msg};

pub trait SmtpService {
    fn send_msg(&mut self, account: &AccountConfig, msg: &Msg) -> Result<lettre::Message>;
    fn send_raw_msg(&mut self, envelope: &lettre::address::Envelope, msg: &[u8]) -> Result<()>;
}

pub struct LettreService<'a> {
    account: &'a AccountConfig,
    transport: Option<SmtpTransport>,
}

impl<'a> LettreService<'a> {
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

impl<'a> SmtpService for LettreService<'a> {
    fn send_msg(&mut self, account: &AccountConfig, msg: &Msg) -> Result<lettre::Message> {
        debug!("sending message…");
        let sendable_msg = msg.into_sendable_msg(account)?;
        self.transport()?.send(&sendable_msg)?;
        Ok(sendable_msg)
    }

    fn send_raw_msg(&mut self, envelope: &lettre::address::Envelope, msg: &[u8]) -> Result<()> {
        debug!("sending raw message…");
        self.transport()?.send_raw(envelope, msg)?;
        Ok(())
    }
}

impl<'a> From<&'a AccountConfig> for LettreService<'a> {
    fn from(account: &'a AccountConfig) -> Self {
        debug!("init SMTP service");
        Self {
            account,
            transport: None,
        }
    }
}
