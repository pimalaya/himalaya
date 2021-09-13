use anyhow::Result;
use lettre::{
    self,
    transport::{smtp::client::Tls, smtp::client::TlsParameters, smtp::SmtpTransport},
    Transport,
};

use crate::config::model::Account;

pub fn send(account: &Account, msg: &lettre::Message) -> Result<()> {
    let smtp_relay = if account.smtp_starttls() {
        SmtpTransport::starttls_relay
    } else {
        SmtpTransport::relay
    };

    let tls = TlsParameters::builder(account.smtp_host.to_string())
        .dangerous_accept_invalid_hostnames(account.smtp_insecure())
        .dangerous_accept_invalid_certs(account.smtp_insecure())
        .build()?;
    let tls = if account.smtp_starttls() {
        Tls::Required(tls)
    } else {
        Tls::Wrapper(tls)
    };

    smtp_relay(&account.smtp_host)?
        .port(account.smtp_port)
        .tls(tls)
        .credentials(account.smtp_creds()?)
        .build()
        .send(msg)?;

    Ok(())
}
