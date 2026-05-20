// This file is part of Himalaya, a CLI to manage emails.
//
// Copyright (C) 2022-2026 soywod <pimalaya.org@posteo.net>
//
// This program is free software: you can redistribute it and/or modify it under
// the terms of the GNU Affero General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option) any
// later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more
// details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Shared converters that turn wizard answers
//! ([`WizardImapConfig`] / [`WizardSmtpConfig`] / [`WizardJmapConfig`])
//! into the on-disk config types ([`ImapConfig`] / [`SmtpConfig`] /
//! [`JmapConfig`]). Used by both [`super::discover`] and
//! [`super::edit`].

use std::process::Command;

use anyhow::{Result, bail};
use pimalaya_cli::wizard::{
    imap::{Encryption as ImapEncryption, ImapAuth, ImapSecret, WizardImapConfig},
    jmap::{JmapAuth, JmapSecret, WizardJmapConfig},
    smtp::{Encryption as SmtpEncryption, SmtpAuth, SmtpSecret, WizardSmtpConfig},
};
use pimalaya_config::{command::shell, secret::Secret};

use crate::config::{
    ImapConfig, JmapAuthConfig, JmapConfig, SaslConfig, SaslPlainConfig, SmtpConfig,
};

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
        sasl,
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
        sasl,
    })
}

pub fn jmap_to_config(w: WizardJmapConfig) -> Result<JmapConfig> {
    let auth = match w.auth {
        JmapAuth::Basic { login, secret } => JmapAuthConfig::Basic {
            username: login,
            password: jmap_secret_to_secret(secret)?,
        },
        JmapAuth::Bearer { secret } => JmapAuthConfig::Bearer {
            token: jmap_secret_to_secret(secret)?,
        },
    };

    Ok(JmapConfig {
        server: w.server,
        tls: Default::default(),
        auth,
        identity_id: None,
        drafts_mailbox_id: None,
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

fn jmap_secret_to_secret(secret: JmapSecret) -> Result<Secret> {
    Ok(match secret {
        JmapSecret::Raw(s) => Secret::Raw(s),
        JmapSecret::Command(cmd) => Secret::Command(parse_cmd(&cmd)?),
    })
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
