use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use himalaya_lib::{
    EmailHooks, EmailSender, EmailTextPlainFormat, MaildirConfig, SendmailConfig, SmtpConfig,
};

#[cfg(feature = "imap-backend")]
use himalaya_lib::ImapConfig;

#[cfg(feature = "notmuch-backend")]
use himalaya_lib::NotmuchConfig;

#[derive(Debug, Default, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(remote = "SmtpConfig")]
struct SmtpConfigDef {
    #[serde(rename = "smtp-host")]
    pub host: String,
    #[serde(rename = "smtp-port")]
    pub port: u16,
    #[serde(rename = "smtp-ssl")]
    pub ssl: Option<bool>,
    #[serde(rename = "smtp-starttls")]
    pub starttls: Option<bool>,
    #[serde(rename = "smtp-insecure")]
    pub insecure: Option<bool>,
    #[serde(rename = "smtp-login")]
    pub login: String,
    #[serde(rename = "smtp-passwd-cmd")]
    pub passwd_cmd: String,
}

#[cfg(feature = "imap-backend")]
#[derive(Debug, Default, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(remote = "ImapConfig")]
pub struct ImapConfigDef {
    #[serde(rename = "imap-host")]
    pub host: String,
    #[serde(rename = "imap-port")]
    pub port: u16,
    #[serde(rename = "imap-ssl")]
    pub ssl: Option<bool>,
    #[serde(rename = "imap-starttls")]
    pub starttls: Option<bool>,
    #[serde(rename = "imap-insecure")]
    pub insecure: Option<bool>,
    #[serde(rename = "imap-login")]
    pub login: String,
    #[serde(rename = "imap-passwd-cmd")]
    pub passwd_cmd: String,
    #[serde(rename = "imap-notify-cmd")]
    pub notify_cmd: Option<String>,
    #[serde(rename = "imap-notify-query")]
    pub notify_query: Option<String>,
    #[serde(rename = "imap-watch-cmds")]
    pub watch_cmds: Option<Vec<String>>,
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(remote = "MaildirConfig")]
pub struct MaildirConfigDef {
    #[serde(rename = "maildir-root-dir")]
    pub root_dir: PathBuf,
}

#[cfg(feature = "notmuch-backend")]
#[derive(Debug, Default, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(remote = "NotmuchConfig")]
pub struct NotmuchConfigDef {
    #[serde(rename = "notmuch-db-path")]
    pub db_path: PathBuf,
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(remote = "Option<EmailTextPlainFormat>")]
pub enum EmailTextPlainFormatOptionDef {
    #[serde(with = "EmailTextPlainFormatDef")]
    Some(EmailTextPlainFormat),
    #[default]
    None,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(remote = "EmailTextPlainFormat", rename_all = "snake_case")]
pub enum EmailTextPlainFormatDef {
    Auto,
    Flowed,
    Fixed(usize),
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(remote = "EmailSender", tag = "sender", rename_all = "snake_case")]
pub enum EmailSenderDef {
    None,
    #[serde(with = "SmtpConfigDef")]
    Smtp(SmtpConfig),
    #[serde(with = "SendmailConfigDef")]
    Sendmail(SendmailConfig),
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(remote = "SendmailConfig")]
pub struct SendmailConfigDef {
    #[serde(rename = "sendmail-cmd")]
    cmd: String,
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(remote = "Option<EmailHooks>")]
pub enum EmailHooksOptionDef {
    #[serde(with = "EmailHooksDef")]
    Some(EmailHooks),
    #[default]
    None,
}

/// Represents the email hooks. Useful for doing extra email
/// processing before or after sending it.
#[derive(Debug, Default, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(remote = "EmailHooks")]
pub struct EmailHooksDef {
    /// Represents the hook called just before sending an email.
    pub pre_send: Option<String>,
}
