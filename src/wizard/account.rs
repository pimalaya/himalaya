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
use pimalaya_stream::sasl::SaslMechanism;

use crate::config::{
    ImapConfig, SaslAnonymousConfig, SaslConfig, SaslLoginConfig, SaslOauthbearerConfig,
    SaslPlainConfig, SaslScramSha256Config, SaslXoauth2Config, SmtpConfig,
};

pub fn imap_to_config(w: WizardImapConfig, mechanism: SaslMechanism) -> Result<ImapConfig> {
    let scheme = match w.encryption {
        ImapEncryption::Tls => "imaps",
        ImapEncryption::StartTls | ImapEncryption::None => "imap",
    };
    let server = format!("{scheme}://{}:{}", w.host, w.port);
    let starttls = matches!(w.encryption, ImapEncryption::StartTls);
    let sasl = Some(build_sasl_imap(&w.login, w.auth, mechanism)?);

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

/// Builds the IMAP SASL config for `mechanism` from the manual wizard's
/// answers. The wizard collects a single secret (labelled a password),
/// which backs whichever mechanism was chosen: the password families use
/// it as the password, the OAuth families as the API token, and
/// ANONYMOUS ignores it.
fn build_sasl_imap(login: &str, auth: ImapAuth, mechanism: SaslMechanism) -> Result<SaslConfig> {
    let ImapAuth::Password(secret) = auth;
    let secret = match secret {
        ImapSecret::Raw(s) => Secret::Raw(s),
        ImapSecret::Command(argv) => command_secret(argv)?,
        ImapSecret::Shell(line) => shell_secret(&line)?,
    };

    Ok(match mechanism {
        SaslMechanism::Plain => SaslConfig::Plain(SaslPlainConfig {
            authzid: None,
            authcid: login.to_owned(),
            passwd: secret,
        }),
        SaslMechanism::Login => SaslConfig::Login(SaslLoginConfig {
            username: login.to_owned(),
            password: secret,
        }),
        SaslMechanism::ScramSha256 => SaslConfig::ScramSha256(SaslScramSha256Config {
            username: login.to_owned(),
            password: secret,
        }),
        SaslMechanism::OAuthBearer => SaslConfig::Oauthbearer(SaslOauthbearerConfig {
            username: login.to_owned(),
            token: secret,
        }),
        SaslMechanism::XOAuth2 => SaslConfig::Xoauth2(SaslXoauth2Config {
            username: login.to_owned(),
            token: secret,
        }),
        SaslMechanism::Anonymous => SaslConfig::Anonymous(SaslAnonymousConfig { message: None }),
    })
}

fn build_sasl_smtp(login: &str, auth: SmtpAuth) -> Result<SaslConfig> {
    let SmtpAuth::Password(secret) = auth;
    let passwd = match secret {
        SmtpSecret::Raw(s) => Secret::Raw(s),
        SmtpSecret::Command(argv) => command_secret(argv)?,
        SmtpSecret::Shell(line) => shell_secret(&line)?,
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

/// Builds a [`Secret::Command`] from an argv (program + arguments, no
/// shell), the form a known keyring provider or token broker yields. It
/// serializes back as a TOML array. Shared with [`super::secret`].
pub(crate) fn command_secret(argv: Vec<String>) -> Result<Secret> {
    let Some((program, args)) = argv.split_first() else {
        bail!("Empty command for secret");
    };

    let mut cmd = Command::new(program);
    cmd.args(args);
    Ok(Secret::Command(cmd))
}

/// Builds a [`Secret::Command`] from a shell command line, the fallback
/// form a user typed by hand. It serializes back as a TOML string.
/// Shared with [`super::secret`].
pub(crate) fn shell_secret(line: &str) -> Result<Secret> {
    let line = line.trim();
    if line.is_empty() {
        bail!("Empty shell command for secret");
    }

    Ok(Secret::Command(shell(line)))
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
            auth: ImapAuth::Password(ImapSecret::Command(
                ["pass", "show", "example"].map(String::from).to_vec(),
            )),
        }
    }

    #[test]
    fn imap_tls_yields_imaps_scheme_and_plain_sasl() {
        let config = imap_to_config(imap(ImapEncryption::Tls, 993), SaslMechanism::Plain).unwrap();
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
        let config =
            imap_to_config(imap(ImapEncryption::StartTls, 143), SaslMechanism::Plain).unwrap();
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
            auth: SmtpAuth::Password(SmtpSecret::Command(
                ["pass", "show", "example"].map(String::from).to_vec(),
            )),
        };
        let config = smtp_to_config(wizard).unwrap();
        assert_eq!(config.server, "smtps://smtp.example.com:465");
        assert!(!config.starttls);
    }

    #[test]
    fn empty_command_secret_is_rejected() {
        let mut wizard = imap(ImapEncryption::Tls, 993);
        wizard.auth = ImapAuth::Password(ImapSecret::Command(Vec::new()));
        assert!(imap_to_config(wizard, SaslMechanism::Plain).is_err());
    }

    #[test]
    fn blank_shell_secret_is_rejected() {
        let mut wizard = imap(ImapEncryption::Tls, 993);
        wizard.auth = ImapAuth::Password(ImapSecret::Shell("   ".into()));
        assert!(imap_to_config(wizard, SaslMechanism::Plain).is_err());
    }
}
