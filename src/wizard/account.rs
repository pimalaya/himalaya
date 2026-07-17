//! Shared converters that turn wizard answers ([`WizardImapConfig`] /
//! [`WizardSmtpConfig`]) into the on-disk config types ([`ImapConfig`] /
//! [`SmtpConfig`]). Used by [`super::discover`].

use std::process::Command;

use anyhow::{Result, bail};
use pimalaya_cli::wizard::{
    imap::{Encryption as ImapEncryption, ImapAuth, ImapSecret, WizardImapConfig},
    smtp::{Encryption as SmtpEncryption, SmtpAuth, SmtpSecret, WizardSmtpConfig},
};
use pimalaya_config::{command::shell, secret::Secret};

use crate::config::{ImapConfig, SaslConfig, SaslPlainConfig, SmtpConfig};

pub fn imap_to_config(w: WizardImapConfig) -> Result<ImapConfig> {
    let scheme = match w.encryption {
        ImapEncryption::Tls => "imaps",
        ImapEncryption::StartTls | ImapEncryption::None => "imap",
    };
    let server = format!("{scheme}://{}:{}", w.host, w.port);
    let starttls = matches!(w.encryption, ImapEncryption::StartTls);
    let sasl = Some(build_sasl_imap(&w.login, w.auth)?);

    Ok(ImapConfig {
        server,
        tls: Default::default(),
        starttls,
        alpn: io_imap::client::default_alpn(),
        sasl,
        id: Default::default(),
        sort: Default::default(),
    })
}

pub fn smtp_to_config(w: WizardSmtpConfig) -> Result<SmtpConfig> {
    let scheme = match w.encryption {
        SmtpEncryption::Tls => "smtps",
        SmtpEncryption::StartTls | SmtpEncryption::None => "smtp",
    };
    let server = format!("{scheme}://{}:{}", w.host, w.port);
    let starttls = matches!(w.encryption, SmtpEncryption::StartTls);
    let sasl = Some(build_sasl_smtp(&w.login, w.auth)?);

    Ok(SmtpConfig {
        server,
        tls: Default::default(),
        starttls,
        alpn: io_smtp::client::SmtpClientStd::default_alpn(),
        sasl,
    })
}

fn build_sasl_imap(login: &str, auth: ImapAuth) -> Result<SaslConfig> {
    let ImapAuth::Password(secret) = auth;
    let passwd = match secret {
        ImapSecret::Raw(s) => Secret::Raw(s),
        ImapSecret::Command(cmd) => Secret::Command(parse_cmd(&cmd)?),
    };

    Ok(plain_sasl(login, passwd))
}

fn build_sasl_smtp(login: &str, auth: SmtpAuth) -> Result<SaslConfig> {
    let SmtpAuth::Password(secret) = auth;
    let passwd = match secret {
        SmtpSecret::Raw(s) => Secret::Raw(s),
        SmtpSecret::Command(cmd) => Secret::Command(parse_cmd(&cmd)?),
    };

    Ok(plain_sasl(login, passwd))
}

fn plain_sasl(login: &str, passwd: Secret) -> SaslConfig {
    SaslConfig::Plain(SaslPlainConfig {
        authzid: None,
        authcid: login.to_owned(),
        passwd,
    })
}

fn parse_cmd(cmd: &str) -> Result<Command> {
    let line = cmd.trim();
    if line.is_empty() {
        bail!("Empty shell command for secret");
    }
    Ok(shell(line))
}
