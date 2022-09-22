use himalaya_lib::{EmailHooks, EmailSendCmd, EmailSender, EmailTextPlainFormat, SmtpConfig};
use serde::Deserialize;
use std::path::PathBuf;

#[cfg(feature = "imap-backend")]
use himalaya_lib::ImapConfig;

#[cfg(feature = "maildir-backend")]
use himalaya_lib::MaildirConfig;

#[cfg(feature = "notmuch-backend")]
use himalaya_lib::NotmuchConfig;

#[derive(Debug, Default, Clone, Eq, PartialEq, Deserialize)]
#[serde(remote = "SmtpConfig")]
struct SmtpConfigDef {
    #[serde(rename = "smtp-host")]
    pub host: String,
    #[serde(rename = "smtp-port")]
    pub port: u16,
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
#[derive(Debug, Default, Clone, Eq, PartialEq, Deserialize)]
#[serde(remote = "ImapConfig")]
pub struct ImapConfigDef {
    #[serde(rename = "imap-host")]
    pub host: String,
    #[serde(rename = "imap-port")]
    pub port: u16,
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

#[cfg(feature = "maildir-backend")]
#[derive(Debug, Default, Clone, Eq, PartialEq, Deserialize)]
#[serde(remote = "MaildirConfig")]
pub struct MaildirConfigDef {
    #[serde(rename = "maildir-root-dir")]
    pub root_dir: PathBuf,
}

#[cfg(feature = "notmuch-backend")]
#[derive(Debug, Default, Clone, Eq, PartialEq, Deserialize)]
#[serde(remote = "NotmuchConfig")]
pub struct NotmuchConfigDef {
    #[serde(rename = "notmuch-db-path")]
    pub db_path: PathBuf,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(remote = "EmailTextPlainFormat", rename_all = "snake_case")]
enum EmailTextPlainFormatDef {
    Auto,
    Flowed,
    Fixed(usize),
}

pub mod email_text_plain_format {
    use himalaya_lib::EmailTextPlainFormat;
    use serde::{Deserialize, Deserializer};

    use super::EmailTextPlainFormatDef;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<EmailTextPlainFormat>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper(#[serde(with = "EmailTextPlainFormatDef")] EmailTextPlainFormat);

        let helper = Option::deserialize(deserializer)?;
        Ok(helper.map(|Helper(external)| external))
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(remote = "EmailSender", tag = "sender", rename_all = "snake_case")]
pub enum EmailSenderDef {
    None,
    #[serde(with = "SmtpConfigDef")]
    Internal(SmtpConfig),
    #[serde(with = "EmailSendCmdDef")]
    External(EmailSendCmd),
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(remote = "EmailSendCmd")]
pub struct EmailSendCmdDef {
    #[serde(rename = "send-cmd")]
    cmd: String,
}

/// Represents the email hooks. Useful for doing extra email
/// processing before or after sending it.
#[derive(Debug, Default, Clone, Eq, PartialEq, Deserialize)]
#[serde(remote = "EmailHooks")]
struct EmailHooksDef {
    /// Represents the hook called just before sending an email.
    pub pre_send: Option<String>,
}

pub mod email_hooks {
    use himalaya_lib::EmailHooks;
    use serde::{Deserialize, Deserializer};

    use super::EmailHooksDef;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<EmailHooks>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper(#[serde(with = "EmailHooksDef")] EmailHooks);

        let helper = Option::deserialize(deserializer)?;
        Ok(helper.map(|Helper(external)| external))
    }
}
