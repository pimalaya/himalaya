use anyhow::{Context, Result};
use lettre::{
    self,
    transport::smtp::{
        client::{Tls, TlsParameters},
        SmtpTransport,
    },
    Transport,
};
use std::convert::TryInto;

use crate::{config::AccountConfig, msg::Msg, output::pipe_cmd};

pub trait SmtpService {
    fn send(&mut self, account: &AccountConfig, msg: &Msg) -> Result<Vec<u8>>;
}

pub struct LettreService<'a> {
    account: &'a AccountConfig,
    transport: Option<SmtpTransport>,
}

impl LettreService<'_> {
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

impl SmtpService for LettreService<'_> {
    fn send(&mut self, account: &AccountConfig, msg: &Msg) -> Result<Vec<u8>> {
        let envelope: lettre::address::Envelope = msg.try_into()?;
        let mut msg = msg.into_sendable_msg(account)?.formatted();

        if let Some(cmd) = account.hooks.pre_send.as_deref() {
            for cmd in cmd.split('|') {
                msg = pipe_cmd(cmd.trim(), &msg)
                    .with_context(|| format!("cannot execute pre-send hook {:?}", cmd))?
            }
        };

        self.transport()?.send_raw(&envelope, &msg)?;
        Ok(msg)
    }
}

impl<'a> From<&'a AccountConfig> for LettreService<'a> {
    fn from(account: &'a AccountConfig) -> Self {
        Self {
            account,
            transport: None,
        }
    }
}
