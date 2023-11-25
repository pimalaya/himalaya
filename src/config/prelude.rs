#[cfg(feature = "pgp-commands")]
use email::account::CmdsPgpConfig;
#[cfg(feature = "pgp-gpg")]
use email::account::GpgConfig;
#[cfg(feature = "pgp")]
use email::account::PgpConfig;
#[cfg(feature = "pgp-native")]
use email::account::{NativePgpConfig, NativePgpSecretKey, SignedSecretKey};
#[cfg(feature = "notmuch-backend")]
use email::backend::NotmuchConfig;
#[cfg(feature = "imap-backend")]
use email::imap::{ImapAuthConfig, ImapConfig};
#[cfg(feature = "smtp-sender")]
use email::smtp::{SmtpAuthConfig, SmtpConfig};
use email::{
    account::{OAuth2Config, OAuth2Method, OAuth2Scopes, PasswdConfig},
    email::{EmailHooks, EmailTextPlainFormat},
    folder::sync::FolderSyncStrategy,
    maildir::MaildirConfig,
    sendmail::SendmailConfig,
};
use keyring::Entry;
use process::{Cmd, Pipeline, SingleCmd};
use secret::Secret;
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
#[serde(remote = "Option<ImapConfig>", from = "OptionImapConfig")]
pub struct OptionImapConfigDef;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct OptionImapConfig {
    #[serde(default, skip)]
    is_none: bool,
    #[serde(flatten, with = "ImapConfigDef")]
    inner: ImapConfig,
}

impl From<OptionImapConfig> for Option<ImapConfig> {
    fn from(config: OptionImapConfig) -> Option<ImapConfig> {
        if config.is_none {
            None
        } else {
            Some(config.inner)
        }
    }
}

#[cfg(feature = "imap-backend")]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "ImapConfig", rename_all = "kebab-case")]
pub struct ImapConfigDef {
    pub host: String,
    pub port: u16,
    pub ssl: Option<bool>,
    pub starttls: Option<bool>,
    pub insecure: Option<bool>,
    pub login: String,
    #[serde(flatten, with = "ImapAuthConfigDef")]
    pub auth: ImapAuthConfig,
    pub notify_cmd: Option<String>,
    pub notify_query: Option<String>,
    pub watch_cmds: Option<Vec<String>>,
}

#[cfg(feature = "imap-backend")]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "ImapAuthConfig", tag = "auth")]
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
#[serde(remote = "Option<MaildirConfig>", from = "OptionMaildirConfig")]
pub struct OptionMaildirConfigDef;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OptionMaildirConfig {
    #[default]
    #[serde(skip_serializing)]
    None,
    Some(#[serde(with = "MaildirConfigDef")] MaildirConfig),
}

impl From<OptionMaildirConfig> for Option<MaildirConfig> {
    fn from(config: OptionMaildirConfig) -> Option<MaildirConfig> {
        match config {
            OptionMaildirConfig::None => None,
            OptionMaildirConfig::Some(config) => Some(config),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "MaildirConfig", rename_all = "kebab-case")]
pub struct MaildirConfigDef {
    #[serde(rename = "maildir-root-dir")]
    pub root_dir: PathBuf,
}

#[cfg(feature = "notmuch-backend")]
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Option<NotmuchConfig>", from = "OptionNotmuchConfig")]
pub struct OptionNotmuchConfigDef;

#[cfg(feature = "notmuch-backend")]
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OptionNotmuchConfig {
    #[default]
    #[serde(skip_serializing)]
    None,
    Some(#[serde(with = "NotmuchConfigDef")] NotmuchConfig),
}

#[cfg(feature = "notmuch-backend")]
impl From<OptionNotmuchConfig> for Option<NotmuchConfig> {
    fn from(config: OptionNotmuchConfig) -> Option<NotmuchConfig> {
        match config {
            OptionNotmuchConfig::None => None,
            OptionNotmuchConfig::Some(config) => Some(config),
        }
    }
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
    remote = "Option<EmailTextPlainFormat>",
    from = "OptionEmailTextPlainFormat"
)]
pub struct OptionEmailTextPlainFormatDef;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OptionEmailTextPlainFormat {
    #[serde(skip_serializing)]
    None,
    #[default]
    Auto,
    Flowed,
    Fixed(usize),
}

impl From<OptionEmailTextPlainFormat> for Option<EmailTextPlainFormat> {
    fn from(fmt: OptionEmailTextPlainFormat) -> Option<EmailTextPlainFormat> {
        match fmt {
            OptionEmailTextPlainFormat::None => None,
            OptionEmailTextPlainFormat::Auto => Some(EmailTextPlainFormat::Auto),
            OptionEmailTextPlainFormat::Flowed => Some(EmailTextPlainFormat::Flowed),
            OptionEmailTextPlainFormat::Fixed(size) => Some(EmailTextPlainFormat::Fixed(size)),
        }
    }
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
#[serde(remote = "Option<SmtpConfig>", from = "OptionSmtpConfig")]
pub struct OptionSmtpConfigDef;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OptionSmtpConfig {
    #[default]
    #[serde(skip_serializing)]
    None,
    Some(#[serde(with = "SmtpConfigDef")] SmtpConfig),
}

impl From<OptionSmtpConfig> for Option<SmtpConfig> {
    fn from(config: OptionSmtpConfig) -> Option<SmtpConfig> {
        match config {
            OptionSmtpConfig::None => None,
            OptionSmtpConfig::Some(config) => Some(config),
        }
    }
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

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Option<SendmailConfig>", from = "OptionSendmailConfig")]
pub struct OptionSendmailConfigDef;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OptionSendmailConfig {
    #[default]
    #[serde(skip_serializing)]
    None,
    Some(#[serde(with = "SendmailConfigDef")] SendmailConfig),
}

impl From<OptionSendmailConfig> for Option<SendmailConfig> {
    fn from(config: OptionSendmailConfig) -> Option<SendmailConfig> {
        match config {
            OptionSendmailConfig::None => None,
            OptionSendmailConfig::Some(config) => Some(config),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "SendmailConfig", rename_all = "kebab-case")]
pub struct SendmailConfigDef {
    #[serde(
        rename = "sendmail-cmd",
        with = "CmdDef",
        default = "sendmail_default_cmd"
    )]
    cmd: Cmd,
}

fn sendmail_default_cmd() -> Cmd {
    Cmd::from("/usr/sbin/sendmail")
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Option<EmailHooks>", from = "OptionEmailHooks")]
pub struct OptionEmailHooksDef;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OptionEmailHooks {
    #[default]
    #[serde(skip_serializing)]
    None,
    Some(#[serde(with = "EmailHooksDef")] EmailHooks),
}

impl From<OptionEmailHooks> for Option<EmailHooks> {
    fn from(fmt: OptionEmailHooks) -> Option<EmailHooks> {
        match fmt {
            OptionEmailHooks::None => None,
            OptionEmailHooks::Some(hooks) => Some(hooks),
        }
    }
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
#[serde(
    remote = "Option<FolderSyncStrategy>",
    from = "OptionFolderSyncStrategy"
)]
pub struct OptionFolderSyncStrategyDef;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OptionFolderSyncStrategy {
    #[default]
    #[serde(skip_serializing)]
    None,
    Some(#[serde(with = "FolderSyncStrategyDef")] FolderSyncStrategy),
}

impl From<OptionFolderSyncStrategy> for Option<FolderSyncStrategy> {
    fn from(config: OptionFolderSyncStrategy) -> Option<FolderSyncStrategy> {
        match config {
            OptionFolderSyncStrategy::None => None,
            OptionFolderSyncStrategy::Some(config) => Some(config),
        }
    }
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

#[cfg(feature = "pgp")]
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "PgpConfig", tag = "backend", rename_all = "kebab-case")]
pub enum PgpConfigDef {
    #[default]
    None,
    #[cfg(feature = "pgp-commands")]
    #[serde(with = "CmdsPgpConfigDef", alias = "commands")]
    Cmds(CmdsPgpConfig),
    #[cfg(feature = "pgp-gpg")]
    #[serde(with = "GpgConfigDef")]
    Gpg(GpgConfig),
    #[cfg(feature = "pgp-native")]
    #[serde(with = "NativePgpConfigDef")]
    Native(NativePgpConfig),
}

#[cfg(feature = "pgp-gpg")]
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "GpgConfig", rename_all = "kebab-case")]
pub struct GpgConfigDef;

#[cfg(feature = "pgp-commands")]
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "CmdsPgpConfig", rename_all = "kebab-case")]
pub struct CmdsPgpConfigDef {
    #[serde(default, with = "OptionCmdDef")]
    encrypt_cmd: Option<Cmd>,
    #[serde(default)]
    encrypt_recipient_fmt: Option<String>,
    #[serde(default)]
    encrypt_recipients_sep: Option<String>,
    #[serde(default, with = "OptionCmdDef")]
    decrypt_cmd: Option<Cmd>,
    #[serde(default, with = "OptionCmdDef")]
    sign_cmd: Option<Cmd>,
    #[serde(default, with = "OptionCmdDef")]
    verify_cmd: Option<Cmd>,
}

#[cfg(feature = "pgp-native")]
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "NativePgpConfig", rename_all = "kebab-case")]
pub struct NativePgpConfigDef {
    #[serde(default, with = "NativePgpSecretKeyDef")]
    secret_key: NativePgpSecretKey,
    #[serde(default, with = "SecretDef")]
    secret_key_passphrase: Secret,
    #[serde(default = "NativePgpConfig::default_wkd")]
    wkd: bool,
    #[serde(default = "NativePgpConfig::default_key_servers")]
    key_servers: Vec<String>,
}

#[cfg(feature = "pgp-native")]
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(remote = "NativePgpSecretKey", rename_all = "kebab-case")]
pub enum NativePgpSecretKeyDef {
    #[default]
    None,
    #[serde(skip)]
    Raw(SignedSecretKey),
    Path(PathBuf),
    #[serde(with = "EntryDef")]
    Keyring(Entry),
}
