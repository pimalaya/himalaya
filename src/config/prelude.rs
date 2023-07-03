#[cfg(feature = "imap-backend")]
use pimalaya_email::backend::{ImapAuthConfig, ImapConfig};
#[cfg(feature = "smtp-sender")]
use pimalaya_email::sender::{SmtpAuthConfig, SmtpConfig};
#[cfg(feature = "notmuch-backend")]
use pimalaya_email::NotmuchConfig;
use pimalaya_email::{
    account::{OAuth2Config, OAuth2Method, OAuth2Scopes, PasswdConfig},
    backend::{BackendConfig, MaildirConfig},
    email::{EmailHooks, EmailTextPlainFormat},
    folder::sync::FolderSyncStrategy,
    sender::{SenderConfig, SendmailConfig},
};
use pimalaya_keyring::Entry;
use pimalaya_process::{Cmd, Pipeline, SingleCmd};
use pimalaya_secret::Secret;
use serde::{ser::SerializeSeq, Deserialize, Serialize, Serializer};
use std::{collections::HashSet, ops::Deref, path::PathBuf};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Entry", from = "String")]
pub struct EntryDef(#[serde(getter = "Deref::deref")] String);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "SingleCmd", from = "String")]
pub struct SingleCmdDef(#[serde(getter = "Deref::deref")] String);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Pipeline", from = "Vec<String>")]
pub struct PipelineDef(
    #[serde(getter = "Deref::deref", serialize_with = "pipeline")] Vec<SingleCmd>,
);

// NOTE: did not find the way to do it with macros…
pub fn pipeline<S>(cmds: &Vec<SingleCmd>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut seq = s.serialize_seq(Some(cmds.len()))?;
    for cmd in cmds {
        seq.serialize_element(&cmd.to_string())?;
    }
    seq.end()
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Cmd", untagged)]
pub enum CmdDef {
    #[serde(with = "SingleCmdDef")]
    SingleCmd(SingleCmd),
    #[serde(with = "PipelineDef")]
    Pipeline(Pipeline),
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Option<Cmd>", from = "OptionCmd")]
pub struct OptionCmdDef;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OptionCmd {
    #[default]
    #[serde(skip_serializing)]
    None,
    #[serde(with = "SingleCmdDef")]
    SingleCmd(SingleCmd),
    #[serde(with = "PipelineDef")]
    Pipeline(Pipeline),
}

impl From<OptionCmd> for Option<Cmd> {
    fn from(cmd: OptionCmd) -> Option<Cmd> {
        match cmd {
            OptionCmd::None => None,
            OptionCmd::SingleCmd(cmd) => Some(Cmd::SingleCmd(cmd)),
            OptionCmd::Pipeline(pipeline) => Some(Cmd::Pipeline(pipeline)),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Secret", rename_all = "kebab-case")]
pub enum SecretDef {
    Raw(String),
    #[serde(with = "CmdDef")]
    Cmd(Cmd),
    #[serde(with = "EntryDef", rename = "keyring")]
    KeyringEntry(Entry),
    #[default]
    Undefined,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "OAuth2Method")]
pub enum OAuth2MethodDef {
    #[serde(rename = "xoauth2", alias = "XOAUTH2")]
    XOAuth2,
    #[serde(rename = "oauthbearer", alias = "OAUTHBEARER")]
    OAuthBearer,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "BackendConfig", tag = "backend", rename_all = "kebab-case")]
pub enum BackendConfigDef {
    #[default]
    None,
    #[cfg(feature = "imap-backend")]
    #[serde(with = "ImapConfigDef")]
    Imap(ImapConfig),
    #[serde(with = "MaildirConfigDef")]
    Maildir(MaildirConfig),
    #[cfg(feature = "notmuch-backend")]
    #[serde(with = "NotmuchConfigDef")]
    Notmuch(NotmuchConfig),
}

#[cfg(feature = "imap-backend")]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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

#[cfg(feature = "imap-backend")]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "ImapAuthConfig", tag = "imap-auth")]
pub enum ImapAuthConfigDef {
    #[serde(rename = "passwd", alias = "password", with = "ImapPasswdConfigDef")]
    Passwd(#[serde(default)] PasswdConfig),
    #[serde(rename = "oauth2", with = "ImapOAuth2ConfigDef")]
    OAuth2(OAuth2Config),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "PasswdConfig")]
pub struct ImapPasswdConfigDef {
    #[serde(
        rename = "imap-passwd",
        with = "SecretDef",
        default,
        skip_serializing_if = "Secret::is_undefined"
    )]
    pub passwd: Secret,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "OAuth2Config")]
pub struct ImapOAuth2ConfigDef {
    #[serde(rename = "imap-oauth2-method", with = "OAuth2MethodDef", default)]
    pub method: OAuth2Method,
    #[serde(rename = "imap-oauth2-client-id")]
    pub client_id: String,
    #[serde(
        rename = "imap-oauth2-client-secret",
        with = "SecretDef",
        default,
        skip_serializing_if = "Secret::is_undefined"
    )]
    pub client_secret: Secret,
    #[serde(rename = "imap-oauth2-auth-url")]
    pub auth_url: String,
    #[serde(rename = "imap-oauth2-token-url")]
    pub token_url: String,
    #[serde(
        rename = "imap-oauth2-access-token",
        with = "SecretDef",
        default,
        skip_serializing_if = "Secret::is_undefined"
    )]
    pub access_token: Secret,
    #[serde(
        rename = "imap-oauth2-refresh-token",
        with = "SecretDef",
        default,
        skip_serializing_if = "Secret::is_undefined"
    )]
    pub refresh_token: Secret,
    #[serde(flatten, with = "ImapOAuth2ScopesDef")]
    pub scopes: OAuth2Scopes,
    #[serde(rename = "imap-oauth2-pkce", default)]
    pub pkce: bool,
    #[serde(
        rename = "imap-oauth2-redirect-host",
        default = "OAuth2Config::default_redirect_host"
    )]
    pub redirect_host: String,
    #[serde(
        rename = "imap-oauth2-redirect-port",
        default = "OAuth2Config::default_redirect_port"
    )]
    pub redirect_port: u16,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "OAuth2Scopes")]
pub enum ImapOAuth2ScopesDef {
    #[serde(rename = "imap-oauth2-scope")]
    Scope(String),
    #[serde(rename = "imap-oauth2-scopes")]
    Scopes(Vec<String>),
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "MaildirConfig", rename_all = "kebab-case")]
pub struct MaildirConfigDef {
    #[serde(rename = "maildir-root-dir")]
    pub root_dir: PathBuf,
}

#[cfg(feature = "notmuch-backend")]
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "NotmuchConfig", rename_all = "kebab-case")]
pub struct NotmuchConfigDef {
    #[serde(rename = "notmuch-db-path")]
    pub db_path: PathBuf,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "SenderConfig", tag = "sender", rename_all = "kebab-case")]
pub enum SenderConfigDef {
    #[default]
    None,
    #[cfg(feature = "smtp-sender")]
    #[serde(with = "SmtpConfigDef")]
    Smtp(SmtpConfig),
    #[serde(with = "SendmailConfigDef")]
    Sendmail(SendmailConfig),
}

#[cfg(feature = "smtp-sender")]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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

#[cfg(feature = "smtp-sender")]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "SmtpAuthConfig", tag = "smtp-auth")]
pub enum SmtpAuthConfigDef {
    #[serde(rename = "passwd", alias = "password", with = "SmtpPasswdConfigDef")]
    Passwd(#[serde(default)] PasswdConfig),
    #[serde(rename = "oauth2", with = "SmtpOAuth2ConfigDef")]
    OAuth2(OAuth2Config),
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "PasswdConfig", default)]
pub struct SmtpPasswdConfigDef {
    #[serde(
        rename = "smtp-passwd",
        with = "SecretDef",
        default,
        skip_serializing_if = "Secret::is_undefined"
    )]
    pub passwd: Secret,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "OAuth2Config")]
pub struct SmtpOAuth2ConfigDef {
    #[serde(rename = "smtp-oauth2-method", with = "OAuth2MethodDef", default)]
    pub method: OAuth2Method,
    #[serde(rename = "smtp-oauth2-client-id")]
    pub client_id: String,
    #[serde(
        rename = "smtp-oauth2-client-secret",
        with = "SecretDef",
        default,
        skip_serializing_if = "Secret::is_undefined"
    )]
    pub client_secret: Secret,
    #[serde(rename = "smtp-oauth2-auth-url")]
    pub auth_url: String,
    #[serde(rename = "smtp-oauth2-token-url")]
    pub token_url: String,
    #[serde(
        rename = "smtp-oauth2-access-token",
        with = "SecretDef",
        default,
        skip_serializing_if = "Secret::is_undefined"
    )]
    pub access_token: Secret,
    #[serde(
        rename = "smtp-oauth2-refresh-token",
        with = "SecretDef",
        default,
        skip_serializing_if = "Secret::is_undefined"
    )]
    pub refresh_token: Secret,
    #[serde(flatten, with = "SmtpOAuth2ScopesDef")]
    pub scopes: OAuth2Scopes,
    #[serde(rename = "smtp-oauth2-pkce", default)]
    pub pkce: bool,
    #[serde(
        rename = "imap-oauth2-redirect-host",
        default = "OAuth2Config::default_redirect_host"
    )]
    pub redirect_host: String,
    #[serde(
        rename = "imap-oauth2-redirect-port",
        default = "OAuth2Config::default_redirect_port"
    )]
    pub redirect_port: u16,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "OAuth2Scopes")]
pub enum SmtpOAuth2ScopesDef {
    #[serde(rename = "smtp-oauth2-scope")]
    Scope(String),
    #[serde(rename = "smtp-oauth2-scopes")]
    Scopes(Vec<String>),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "SendmailConfig", rename_all = "kebab-case")]
pub struct SendmailConfigDef {
    #[serde(rename = "sendmail-cmd", with = "CmdDef")]
    cmd: Cmd,
}

/// Represents the email hooks. Useful for doing extra email
/// processing before or after sending it.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "EmailHooks", rename_all = "kebab-case")]
pub struct EmailHooksDef {
    /// Represents the hook called just before sending an email.
    #[serde(default, with = "OptionCmdDef")]
    pub pre_send: Option<Cmd>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "FolderSyncStrategy", rename_all = "kebab-case")]
pub enum FolderSyncStrategyDef {
    #[default]
    All,
    #[serde(alias = "only")]
    Include(HashSet<String>),
    #[serde(alias = "except")]
    #[serde(alias = "ignore")]
    Exclude(HashSet<String>),
}
