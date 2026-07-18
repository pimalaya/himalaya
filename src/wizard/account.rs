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

#[cfg(test)]
mod tests {
    use super::*;

    fn imap(encryption: ImapEncryption, port: u16) -> WizardImapConfig {
        WizardImapConfig {
            host: "imap.example.com".into(),
            port,
            encryption,
            login: "user@example.com".into(),
            auth: ImapAuth::Password(ImapSecret::Command("pass show example".into())),
        }
    }

    #[test]
    fn imap_tls_yields_imaps_scheme_and_plain_sasl() {
        let config = imap_to_config(imap(ImapEncryption::Tls, 993)).unwrap();
        assert_eq!(config.server, "imaps://imap.example.com:993");
        assert!(!config.starttls);
        let Some(SaslConfig::Plain(plain)) = config.sasl else {
            panic!("expected a plain SASL config");
        };
        assert_eq!(plain.authcid, "user@example.com");
        assert!(matches!(plain.passwd, Secret::Command(_)));
    }

    #[test]
    fn imap_starttls_yields_imap_scheme_and_flag() {
        let config = imap_to_config(imap(ImapEncryption::StartTls, 143)).unwrap();
        assert_eq!(config.server, "imap://imap.example.com:143");
        assert!(config.starttls);
    }

    #[test]
    fn smtp_tls_yields_smtps_scheme() {
        let wizard = WizardSmtpConfig {
            host: "smtp.example.com".into(),
            port: 465,
            encryption: SmtpEncryption::Tls,
            login: "user@example.com".into(),
            auth: SmtpAuth::Password(SmtpSecret::Command("pass show example".into())),
        };
        let config = smtp_to_config(wizard).unwrap();
        assert_eq!(config.server, "smtps://smtp.example.com:465");
        assert!(!config.starttls);
    }

    #[test]
    fn blank_secret_command_is_rejected() {
        let mut wizard = imap(ImapEncryption::Tls, 993);
        wizard.auth = ImapAuth::Password(ImapSecret::Command("   ".into()));
        assert!(imap_to_config(wizard).is_err());
    }
}
