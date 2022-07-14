use anyhow::{Context, Result};
use himalaya_lib::{account::Account, msg::Msg};
use lettre::{
    self,
    transport::smtp::{
        client::{Tls, TlsParameters},
        SmtpTransport,
    },
    Transport,
};
use std::convert::TryInto;

use crate::output::pipe_cmd;

pub trait SmtpService {
    fn send(&mut self, account: &Account, msg: &Msg) -> Result<Vec<u8>>;
}

pub struct LettreService<'a> {
    account: &'a Account,
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
    fn send(&mut self, account: &Account, msg: &Msg) -> Result<Vec<u8>> {
        let mut raw_msg = msg.into_sendable_msg(account)?.formatted();

        let envelope: lettre::address::Envelope =
            if let Some(cmd) = account.hooks.pre_send.as_deref() {
                for cmd in cmd.split('|') {
                    raw_msg = pipe_cmd(cmd.trim(), &raw_msg)
                        .with_context(|| format!("cannot execute pre-send hook {:?}", cmd))?;
                }
                let parsed_mail = mailparse::parse_mail(&raw_msg)?;
                Msg::from_parsed_mail(parsed_mail, account)?.try_into()
            } else {
                msg.try_into()
            }?;

        self.transport()?.send_raw(&envelope, &raw_msg)?;
        Ok(raw_msg)
    }
}

impl<'a> From<&'a Account> for LettreService<'a> {
    fn from(account: &'a Account) -> Self {
        Self {
            account,
            transport: None,
        }
    }
}
