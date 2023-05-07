use pimalaya_email::{
    folder::sync::Strategy as SyncFoldersStrategy, EmailHooks, EmailSender, EmailTextPlainFormat,
    ImapAuthConfig, MaildirConfig, OAuth2Config, OAuth2Method, OAuth2Scopes, PasswdConfig,
    SendmailConfig, SmtpAuthConfig, SmtpConfig,
};
use pimalaya_keyring::Entry;
use pimalaya_process::Cmd;
use pimalaya_secret::Secret;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::PathBuf};

#[cfg(feature = "imap-backend")]
use pimalaya_email::ImapConfig;

#[cfg(feature = "notmuch-backend")]
use pimalaya_email::NotmuchConfig;

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(remote = "Entry", from = "String")]
pub struct EntryDef;

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(remote = "Cmd", from = "String")]
pub struct SingleCmdDef;

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(remote = "Cmd", from = "Vec<String>")]
pub struct PipelineDef;

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(remote = "Cmd", from = "SingleCmdOrPipeline")]
pub struct SingleCmdOrPipelineDef;

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum SingleCmdOrPipeline {
    #[serde(with = "SingleCmdDef")]
    SingleCmd(Cmd),
    #[serde(with = "PipelineDef")]
    Pipeline(Cmd),
}

impl From<SingleCmdOrPipeline> for Cmd {
    fn from(cmd: SingleCmdOrPipeline) -> Cmd {
        match cmd {
            SingleCmdOrPipeline::SingleCmd(cmd) => cmd,
            SingleCmdOrPipeline::Pipeline(cmd) => cmd,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(remote = "Secret", rename_all = "kebab-case")]
pub enum SecretDef {
    Raw(String),
    #[serde(with = "SingleCmdOrPipelineDef")]
    Cmd(Cmd),
    #[serde(with = "EntryDef")]
    Keyring(Entry),
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(remote = "OAuth2Method")]
pub enum OAuth2MethodDef {
    #[serde(rename = "xoauth2", alias = "XOAUTH2")]
    XOAuth2,
    #[serde(rename = "oauthbearer", alias = "OAUTHBEARER")]
    OAuthBearer,
}

#[cfg(feature = "imap-backend")]
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
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
    #[serde(flatten, with = "ImapAuthConfigDef")]
    pub auth: ImapAuthConfig,
    #[serde(rename = "imap-notify-cmd")]
    pub notify_cmd: Option<String>,
    #[serde(rename = "imap-notify-query")]
    pub notify_query: Option<String>,
    #[serde(rename = "imap-watch-cmds")]
    pub watch_cmds: Option<Vec<String>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(remote = "ImapAuthConfig", tag = "imap-auth")]
pub enum ImapAuthConfigDef {
    #[serde(rename = "passwd", alias = "password", with = "ImapPasswdConfigDef")]
    Passwd(#[serde(default)] PasswdConfig),
    #[serde(rename = "oauth2", with = "ImapOAuth2ConfigDef")]
    OAuth2(OAuth2Config),
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(remote = "PasswdConfig")]
pub struct ImapPasswdConfigDef {
    #[serde(rename = "imap-passwd", with = "SecretDef", default)]
    pub passwd: Secret,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(remote = "OAuth2Config")]
pub struct ImapOAuth2ConfigDef {
    #[serde(rename = "imap-oauth2-method", with = "OAuth2MethodDef", default)]
    pub method: OAuth2Method,
    #[serde(rename = "imap-oauth2-client-id")]
    pub client_id: String,
    #[serde(rename = "imap-oauth2-client-secret", with = "SecretDef", default)]
    pub client_secret: Secret,
    #[serde(rename = "imap-oauth2-auth-url")]
    pub auth_url: String,
    #[serde(rename = "imap-oauth2-token-url")]
    pub token_url: String,
    #[serde(rename = "imap-oauth2-access-token", with = "SecretDef", default)]
    pub access_token: Secret,
    #[serde(rename = "imap-oauth2-refresh-token", with = "SecretDef", default)]
    pub refresh_token: Secret,
    #[serde(flatten, with = "ImapOAuth2ScopesDef")]
    pub scopes: OAuth2Scopes,
    #[serde(rename = "imap-oauth2-pkce", default)]
    pub pkce: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(remote = "OAuth2Scopes")]
pub enum ImapOAuth2ScopesDef {
    #[serde(rename = "imap-oauth2-scope")]
    Scope(String),
    #[serde(rename = "imap-oauth2-scopes")]
    Scopes(Vec<String>),
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(remote = "MaildirConfig", rename_all = "kebab-case")]
pub struct MaildirConfigDef {
    #[serde(rename = "maildir-root-dir")]
    pub root_dir: PathBuf,
}

#[cfg(feature = "notmuch-backend")]
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(remote = "NotmuchConfig", rename_all = "kebab-case")]
pub struct NotmuchConfigDef {
    #[serde(rename = "notmuch-db-path")]
    pub db_path: PathBuf,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    remote = "EmailTextPlainFormat",
    tag = "type",
    content = "width",
    rename_all = "kebab-case"
)]
pub enum EmailTextPlainFormatDef {
    #[default]
    Auto,
    Flowed,
    Fixed(usize),
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(remote = "EmailSender", tag = "sender", rename_all = "kebab-case")]
pub enum EmailSenderDef {
    #[default]
    None,
    #[serde(with = "SmtpConfigDef")]
    Smtp(SmtpConfig),
    #[serde(with = "SendmailConfigDef")]
    Sendmail(SendmailConfig),
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
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
    #[serde(flatten, with = "SmtpAuthConfigDef")]
    pub auth: SmtpAuthConfig,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(remote = "SmtpAuthConfig", tag = "smtp-auth")]
pub enum SmtpAuthConfigDef {
    #[serde(rename = "passwd", alias = "password", with = "SmtpPasswdConfigDef")]
    Passwd(#[serde(default)] PasswdConfig),
    #[serde(rename = "oauth2", with = "SmtpOAuth2ConfigDef")]
    OAuth2(OAuth2Config),
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(remote = "PasswdConfig", default)]
pub struct SmtpPasswdConfigDef {
    #[serde(rename = "smtp-passwd", with = "SecretDef", default)]
    pub passwd: Secret,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(remote = "OAuth2Config")]
pub struct SmtpOAuth2ConfigDef {
    #[serde(rename = "smtp-oauth2-method", with = "OAuth2MethodDef", default)]
    pub method: OAuth2Method,
    #[serde(rename = "smtp-oauth2-client-id")]
    pub client_id: String,
    #[serde(rename = "smtp-oauth2-client-secret", with = "SecretDef", default)]
    pub client_secret: Secret,
    #[serde(rename = "smtp-oauth2-auth-url")]
    pub auth_url: String,
    #[serde(rename = "smtp-oauth2-token-url")]
    pub token_url: String,
    #[serde(rename = "smtp-oauth2-access-token", with = "SecretDef", default)]
    pub access_token: Secret,
    #[serde(rename = "smtp-oauth2-refresh-token", with = "SecretDef", default)]
    pub refresh_token: Secret,
    #[serde(flatten, with = "SmtpOAuth2ScopesDef")]
    pub scopes: OAuth2Scopes,
    #[serde(rename = "smtp-oauth2-pkce", default)]
    pub pkce: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(remote = "OAuth2Scopes")]
pub enum SmtpOAuth2ScopesDef {
    #[serde(rename = "smtp-oauth2-scope")]
    Scope(String),
    #[serde(rename = "smtp-oauth2-scopes")]
    Scopes(Vec<String>),
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(remote = "SendmailConfig", rename_all = "kebab-case")]
pub struct SendmailConfigDef {
    #[serde(rename = "sendmail-cmd")]
    cmd: String,
}

/// Represents the email hooks. Useful for doing extra email
/// processing before or after sending it.
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(remote = "EmailHooks", rename_all = "kebab-case")]
pub struct EmailHooksDef {
    /// Represents the hook called just before sending an email.
    pub pre_send: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(remote = "SyncFoldersStrategy", rename_all = "kebab-case")]
pub enum SyncFoldersStrategyDef {
    #[default]
    All,
    #[serde(alias = "only")]
    Include(HashSet<String>),
    #[serde(alias = "except")]
    #[serde(alias = "ignore")]
    Exclude(HashSet<String>),
}
